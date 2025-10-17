use tonic::{Request, Response, Status};
use crate::objects::commit::Commit;
use crate::rpc::gitfs::{CommitTreeRequest, CommitTreeResponse, TreeCurrentRequest, TreeCurrentResponse};
use crate::rpc::service::RpcServiceCore;
use crate::sha::HashValue;

#[tonic::async_trait]
impl crate::rpc::gitfs::tree_service_server::TreeService for RpcServiceCore {
    /// Determine the commit that last modified each entry in the tree at a given repository path for a specified revision or refs.
    ///
    /// Given a repository (required) and either an explicit revision hash, a revision string, or refs, this RPC handler:
    /// - resolves the starting commit,
    /// - locates the tree at the normalized path,
    /// - traverses the commit graph backwards to find, for each entry present in the head tree, the most recent commit where that entry was introduced or changed,
    /// - and returns a TreeCurrentResponse whose items pair each tree entry with the commit that last changed it. If the path or tree does not exist, an empty items list is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::TreeCurrentRequest;
    /// # async fn example(core: &crate::rpc::RpcServiceCore) {
    /// let req = Request::new(TreeCurrentRequest {
    ///     repository: None, // fill with a valid RpcRepository in real usage
    ///     revision: "".to_string(),
    ///     refs: "refs/heads/main".to_string(),
    ///     path: "src".to_string(),
    /// });
    /// let _res = core.get_current_tree(req).await;
    /// # }
    /// ```
    async fn get_current_tree(&self, request: Request<TreeCurrentRequest>) -> Result<Response<TreeCurrentResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = crate::rpc::rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let start_commit = if let Some(rev) = inner.revision.clone().filter(|s| !s.is_empty()) {
            if let Some(h) = HashValue::from_str(&rev) {
                repo.odb
                    .get_commit(&h)
                    .await
                    .map_err(|e| Status::internal(format!("failed to get commit by revision hash: {:?}", e)))?
            } else {
                let r = repo
                    .refs
                    .get_refs(rev)
                    .await
                    .map_err(|e| Status::internal(format!("failed to resolve revision refs: {:?}", e)))?;
                repo.odb
                    .get_commit(&r.value)
                    .await
                    .map_err(|e| Status::internal(format!("failed to get commit by resolved revision: {:?}", e)))?
            }
        } else {
            let r = repo
                .refs
                .get_refs(inner.refs)
                .await
                .map_err(|e| Status::internal(format!("failed to get refs: {:?}", e)))?;
            repo.odb
                .get_commit(&r.value)
                .await
                .map_err(|e| Status::internal(format!("failed to get commit: {:?}", e)))?
        };
        let path = normalize_path(inner.path);
        let head_tree = match resolve_tree_at_path(&repo, &start_commit, &path).await {
            Some(t) => t,
            None => {
                return Ok(Response::new(TreeCurrentResponse { items: vec![] }));
            }
        };
        use crate::objects::tree::TreeItem;
        let head_entries: Vec<TreeItem> = head_tree.tree_items.clone();
        if head_entries.is_empty() { return Ok(Response::new(TreeCurrentResponse { items: vec![] })); }
        use std::collections::{HashMap, HashSet, VecDeque};
        let mut assigned: HashMap<String, Commit> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<Commit> = VecDeque::new();
        queue.push_back(start_commit.clone());
        while let Some(c) = queue.pop_front() {
            if assigned.len() >= head_entries.len() { break; }
            let c_hash = c.hash.to_string();
            if !visited.insert(c_hash) { continue; }
            let tree_c = resolve_tree_at_path(&repo, &c, &path).await;
            if c.parents.is_empty() {
                if let Some(t) = tree_c.as_ref() {
                    let names_c: HashSet<&str> = t.tree_items.iter().map(|e| e.name.as_str()).collect();
                    for e in &head_entries {
                        if assigned.contains_key(&e.name) { continue; }
                        if names_c.contains(e.name.as_str()) {
                            assigned.insert(e.name.clone(), c.clone());
                        }
                    }
                }
            } else {
                for p_hash in &c.parents {
                    if let Ok(p_commit) = repo.odb.get_commit(p_hash).await {
                        queue.push_back(p_commit.clone());
                        let tree_p = resolve_tree_at_path(&repo, &p_commit, &path).await;
                        use crate::objects::tree::TreeItemMode;
                        let mut map_c: HashMap<&str, (TreeItemMode, &HashValue)> = HashMap::new();
                        if let Some(t) = tree_c.as_ref() {
                            for e in &t.tree_items { map_c.insert(e.name.as_str(), (e.mode, &e.id)); }
                        }
                        let mut map_p: HashMap<&str, (TreeItemMode, &HashValue)> = HashMap::new();
                        if let Some(t) = tree_p.as_ref() {
                            for e in &t.tree_items { map_p.insert(e.name.as_str(), (e.mode, &e.id)); }
                        }
                        for e in &head_entries {
                            if assigned.contains_key(&e.name) { continue; }
                            let cur = map_c.get(e.name.as_str());
                            let prev = map_p.get(e.name.as_str());
                            let changed = match (prev, cur) {
                                (None, Some((_cm, _cid))) => true,
                                (Some((_pm, _pid)), None) => true,
                                (Some((pm, pid)), Some((cm, cid))) => pm != cm || pid != cid,
                                (None, None) => false,
                            };
                            if changed {
                                assigned.insert(e.name.clone(), c.clone());
                            }
                        }
                    }
                }
            }
        }
        for e in &head_entries {
            if !assigned.contains_key(&e.name) { assigned.insert(e.name.clone(), start_commit.clone()); }
        }
        use crate::rpc::gitfs::{RpcCommit, RpcSignature, RpcTreeItem, RpcTreeItemMode, TreeMessage};
        let mut items = Vec::with_capacity(head_entries.len());
        for e in head_entries {
            let last = assigned.get(&e.name).unwrap_or(&start_commit);
            let rpc_item = RpcTreeItem {
                mode: match e.mode {
                    crate::objects::tree::TreeItemMode::Blob => RpcTreeItemMode::Blob as i32,
                    crate::objects::tree::TreeItemMode::BlobExecutable => RpcTreeItemMode::BlobExecutable as i32,
                    crate::objects::tree::TreeItemMode::Tree => RpcTreeItemMode::Tree as i32,
                    crate::objects::tree::TreeItemMode::Commit => RpcTreeItemMode::Commit as i32,
                    crate::objects::tree::TreeItemMode::Link => RpcTreeItemMode::Link as i32,
                },
                id: e.id.to_string(),
                name: e.name.clone(),
            };
            let rpc_commit = RpcCommit {
                hash: last.hash.to_string(),
                message: last.message.clone(),
                author: Some(RpcSignature {
                    name: last.author.name.clone(),
                    email: last.author.email.clone(),
                    time: last.author.timestamp as i64,
                }),
                committer: Some(RpcSignature {
                    name: last.committer.name.clone(),
                    email: last.committer.email.clone(),
                    time: last.committer.timestamp as i64,
                }),
                parents: last.parents.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
                tree: last.tree.as_ref().map(|x| x.to_string()).unwrap_or_default(),
                gpgsig: last.gpgsig.as_ref().map(|x| x.signature.clone()).unwrap_or_default(),
            };
            items.push(TreeMessage { item: Some(rpc_item), commit: Some(rpc_commit) });
        }
        Ok(Response::new(TreeCurrentResponse { items }))
    }

    /// Resolve the tree at `request.commit_hash` for `request.path` in the given repository and return its RPC representation.
    ///
    /// Returns `Some(RpcTree)` when a tree exists at the specified path for the commit, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::CommitTreeRequest;
    /// # use crate::rpc::RpcServiceCore;
    /// # async fn example(svc: &RpcServiceCore) {
    /// let req = Request::new(CommitTreeRequest {
    ///     repository: None, // fill with a valid RpcRepository
    ///     commit_hash: "012345...".into(),
    ///     path: "src".into(),
    /// });
    /// let resp = svc.get_commit_tree(req).await;
    /// match resp {
    ///     Ok(r) => println!("tree: {:?}", r.into_inner().tree),
    ///     Err(e) => eprintln!("error: {:?}", e),
    /// }
    /// # }
    /// ```
    async fn get_commit_tree(&self, request: Request<CommitTreeRequest>) -> Result<Response<CommitTreeResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = crate::rpc::rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let hash = HashValue::from_str(&inner.commit_hash)
            .ok_or(Status::invalid_argument("invalid commit hash"))?;
        let commit = repo
            .odb
            .get_commit(&hash)
            .await
            .map_err(|e| Status::internal(format!("failed to get commit: {:?}", e)))?;
        let path = normalize_path(inner.path);
        let tree = resolve_tree_at_path(&repo, &commit, &path).await;
        use crate::rpc::gitfs::{RpcTree, RpcTreeItem, RpcTreeItemMode};
        let rpc_tree = match tree {
            Some(t) => Some(RpcTree {
                id: t.id.to_string(),
                tree_items: t.tree_items.into_iter().map(|e| RpcTreeItem {
                    mode: match e.mode {
                        crate::objects::tree::TreeItemMode::Blob => RpcTreeItemMode::Blob as i32,
                        crate::objects::tree::TreeItemMode::BlobExecutable => RpcTreeItemMode::BlobExecutable as i32,
                        crate::objects::tree::TreeItemMode::Tree => RpcTreeItemMode::Tree as i32,
                        crate::objects::tree::TreeItemMode::Commit => RpcTreeItemMode::Commit as i32,
                        crate::objects::tree::TreeItemMode::Link => RpcTreeItemMode::Link as i32,
                    },
                    id: e.id.to_string(),
                    name: e.name,
                }).collect(),
            }),
            None => None,
        };
        Ok(Response::new(CommitTreeResponse { tree: rpc_tree }))
    }
}

/// Normalize a file-system style path for repository tree lookup.
///
/// Converts backslashes to forward slashes and removes any leading or trailing slashes,
/// preserving internal separators. An empty or all-slash input becomes an empty string.
///
/// # Examples
///
/// ```
/// assert_eq!(normalize_path(String::from(r"\a\b\c/")), "a/b/c");
/// assert_eq!(normalize_path(String::from("/")), "");
/// assert_eq!(normalize_path(String::from("dir/subdir")), "dir/subdir");
/// ```
fn normalize_path(path: String) -> String {
    let p = path.replace('\\', "/");
    let p = p.trim_matches('/').to_string();
    p
}

/// Resolve and return the tree object reachable from `commit` at the given slash-separated `path`.
///
/// An empty `path` refers to the commit's root tree. Path segments are matched against tree entries
/// and must correspond to tree (directory) entries; resolution returns `None` if any segment is missing
/// or if an intermediate tree object cannot be loaded from the repository object database.
///
/// # Examples
///
/// ```no_run
/// # use crate::repository::Repository;
/// # use crate::objects::commit::Commit;
/// # async fn example(repo: &Repository, commit: &Commit) {
/// let tree = resolve_tree_at_path(repo, commit, "src/components").await;
/// match tree {
///     Some(t) => { /* found tree at path */ }
///     None => { /* path not found */ }
/// }
/// # }
/// ```
async fn resolve_tree_at_path(
    repo: &crate::repository::Repository,
    commit: &Commit,
    path: &str,
) -> Option<crate::objects::tree::Tree> {
    let cur = commit.tree.clone()?;
    if path.is_empty() { return repo.odb.get_tree(&cur).await.ok(); }
    let mut tree = match repo.odb.get_tree(&cur).await.ok() {
        Some(t) => t,
        None => return None,
    };
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for seg in segments {
        use crate::objects::tree::TreeItemMode;
        let maybe = tree
            .tree_items
            .iter()
            .find(|e| e.name == seg && matches!(e.mode, TreeItemMode::Tree))
            .cloned();
        let entry = match maybe { Some(e) => e, None => return None };
        match repo.odb.get_tree(&entry.id).await.ok() {
            Some(t) => tree = t,
            None => return None,
        }
    }
    Some(tree)
}