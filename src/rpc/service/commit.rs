use dashmap::DashSet;
use tonic::{Request, Response, Status};
use crate::rpc::gitfs::{CommitGetRequest, CommitGetResponse, CommitHeadRequest, CommitHeadResponse, CommitLogRequest, CommitLogResponse, RpcCommit, RpcSignature};
use crate::rpc::rpc_repository_to_inner_repository;
use crate::rpc::service::RpcServiceCore;
use crate::serve::AppCore;
use crate::sha::HashValue;

#[tonic::async_trait]
impl crate::rpc::gitfs::commit_service_server::CommitService for RpcServiceCore {
    /// Retrieve the repository's current HEAD commit.
    ///
    /// A CommitHeadResponse containing the repository's current head commit as an `RpcCommit`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::CommitHeadRequest;
    /// # async fn demo(svc: &crate::rpc::RpcServiceCore) {
    /// let req = Request::new(CommitHeadRequest { repository: None });
    /// let _res = svc.head(req).await;
    /// # }
    /// ```
    async fn head(&self, request: Request<CommitHeadRequest>) -> Result<Response<CommitHeadResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let head = repo
            .refs
            .head()
            .await
            .map_err(|e| Status::internal(format!("failed to get head: {:?}", e)))?;
        let commit = repo
            .odb
            .get_commit(&head.value)
            .await
            .map_err(|e| Status::internal(format!("failed to get commit: {:?}", e)))?;
        Ok(Response::new(CommitHeadResponse {
            commit: Some(RpcCommit {
                hash: commit.hash.to_string(),
                message: commit.message,
                author: Option::from(RpcSignature {
                    name: commit.author.name,
                    email: commit.author.email,
                    time: commit.author.timestamp as i64,
                }),
                committer: Some(RpcSignature {
                    name: commit.committer.name,
                    email: commit.committer.email,
                    time: commit.committer.timestamp as i64,
                }),
                parents: commit.parents.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
                tree: commit.tree.map(|x| x.to_string()).unwrap_or_default(),
                gpgsig: commit.gpgsig.map(|x| x.signature).unwrap_or("".to_string()),
            }),
        }))
    }

    /// Fetches the commit identified by `hash` from the provided repository and returns it as an `RpcCommit` inside a `CommitGetResponse`.
    ///
    /// Returns a `CommitGetResponse` with its `commit` field set to the requested commit on success.
    /// Returns a gRPC `Status` error when the repository is missing, the hash is invalid, or commit retrieval fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::CommitGetRequest;
    /// # async fn example(svc: &crate::rpc::service::RpcServiceCore) {
    /// let req = CommitGetRequest {
    ///     repository: Some(/* RpcRepository */ Default::default()),
    ///     hash: "0123456789abcdef".to_string(),
    /// };
    /// let resp = svc.get(Request::new(req)).await;
    /// match resp {
    ///     Ok(response) => {
    ///         let body = response.into_inner();
    ///         // `body.commit` contains the retrieved commit
    ///     }
    ///     Err(status) => eprintln!("gRPC error: {}", status),
    /// }
    /// # }
    /// ```
    async fn get(&self, request: Request<CommitGetRequest>) -> Result<Response<CommitGetResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let hash = HashValue::from_str(&inner.hash)
            .ok_or(Status::invalid_argument("invalid hash"))?;
        let commit = repo
            .odb
            .get_commit(&hash)
            .await
            .map_err(|e| Status::internal(format!("failed to get commit: {:?}", e)))?;
        Ok(Response::new(CommitGetResponse {
            commit: Some(RpcCommit {
                hash: commit.hash.to_string(),
                message: commit.message,
                author: Option::from(RpcSignature {
                    name: commit.author.name,
                    email: commit.author.email,
                    time: commit.author.timestamp as i64,
                }),
                committer: Some(RpcSignature {
                    name: commit.committer.name,
                    email: commit.committer.email,
                    time: commit.committer.timestamp as i64,
                }),
                parents: commit.parents.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
                tree: commit.tree.map(|x| x.to_string()).unwrap_or_default(),
                gpgsig: commit.gpgsig.map(|x| x.signature).unwrap_or("".to_string()),
            }),
        }))
    }

    /// Traverses commit history from the given reference and returns a paginated list of commits.
    ///
    /// The method resolves the provided repository and reference, performs a breadth-first traversal
    /// of commits starting from that reference, applies the requested `offset` and `limit`, and
    /// returns the collected commits as `RpcCommit` entries in a `CommitLogResponse`.
    ///
    /// # Returns
    ///
    /// `CommitLogResponse` containing the matching commits in traversal order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Perform an asynchronous call to the service's `log` method.
    /// // `service` is an instance of `RpcServiceCore` and `request` is a `tonic::Request<CommitLogRequest>`.
    /// let response = service.log(request).await;
    /// let log_response = response.unwrap().into_inner();
    /// println!("found {} commits", log_response.commits.len());
    /// ```
    async fn log(&self, request: Request<CommitLogRequest>) -> Result<Response<CommitLogResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let refs = repo
            .refs
            .get_refs(inner.r#ref)
            .await
            .map_err(|e| Status::internal(format!("failed to get refs: {:?}", e)))?;
        use std::collections::{HashSet, VecDeque};
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(refs.value);
        let mut result = Vec::new();
        let mut idx = 0;
        while let Some(cmt) = queue.pop_front() {
            if visited.contains(&cmt) {
                continue;
            }
            visited.insert(cmt.clone());
            let commit = repo
                .odb
                .get_commit(&cmt)
                .await
                .map_err(|e| Status::internal(format!("failed to get commit: {:?}", e)))?;
            if idx >= inner.offset {
                result.push(RpcCommit {
                    hash: commit.hash.to_string(),
                    message: commit.message,
                    author: Option::from(RpcSignature {
                        name: commit.author.name,
                        email: commit.author.email,
                        time: commit.author.timestamp as i64,
                    }),
                    committer: Some(RpcSignature {
                        name: commit.committer.name,
                        email: commit.committer.email,
                        time: commit.committer.timestamp as i64,
                    }),
                    parents: commit.parents.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
                    tree: commit.tree.map(|x| x.to_string()).unwrap_or_default(),
                    gpgsig: commit.gpgsig.map(|x| x.signature).unwrap_or("".to_string()),
                });
                if result.len() >= inner.limit as usize {
                    break;
                }
            }
            idx += 1;
            for parent in &commit.parents {
                if !visited.contains(parent) {
                    queue.push_back(parent.clone());
                }
            }
        }
        Ok(Response::new(CommitLogResponse {
            commits: result,
        }))
    }
}