use crate::rpc::gitfs::{
    CommitGetRequest, CommitGetResponse, CommitHeadRequest, CommitHeadResponse, CommitLogRequest,
    CommitLogResponse, RpcCommit, RpcSignature,
};
use crate::rpc::rpc_repository_to_inner_repository;
use crate::rpc::service::RpcServiceCore;
use crate::sha::HashValue;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl crate::rpc::gitfs::commit_service_server::CommitService for RpcServiceCore {
    async fn head(
        &self,
        request: Request<CommitHeadRequest>,
    ) -> Result<Response<CommitHeadResponse>, Status> {
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
                parents: commit
                    .parents
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>(),
                tree: commit.tree.map(|x| x.to_string()).unwrap_or_default(),
                gpgsig: commit.gpgsig.map(|x| x.signature).unwrap_or("".to_string()),
            }),
        }))
    }

    async fn get(
        &self,
        request: Request<CommitGetRequest>,
    ) -> Result<Response<CommitGetResponse>, Status> {
        let inner = request.into_inner();
        let rpc_repo = inner
            .repository
            .ok_or(Status::invalid_argument("missing repository"))?;
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo)
            .await
            .map_err(|e| Status::internal(format!("failed to get repository: {:?}", e)))?;
        let hash =
            HashValue::from_str(&inner.hash).ok_or(Status::invalid_argument("invalid hash"))?;
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
                parents: commit
                    .parents
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>(),
                tree: commit.tree.map(|x| x.to_string()).unwrap_or_default(),
                gpgsig: commit.gpgsig.map(|x| x.signature).unwrap_or("".to_string()),
            }),
        }))
    }

    async fn log(
        &self,
        request: Request<CommitLogRequest>,
    ) -> Result<Response<CommitLogResponse>, Status> {
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
                    parents: commit
                        .parents
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>(),
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
        Ok(Response::new(CommitLogResponse { commits: result }))
    }
}
