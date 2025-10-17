use tonic::{Code, Request, Response, Status};
use crate::error::GitInnerError;
use crate::rpc::gitfs::{RpcRefs, RpcRefsExchangeDefaultRequest, RpcRefsExchangeDefaultResponse, RpcRefsRequest, RpcRefsResponse};
use crate::rpc::rpc_repository_to_inner_repository;
use crate::rpc::service::RpcServiceCore;
use crate::serve::AppCore;


#[tonic::async_trait]
impl crate::rpc::gitfs::refs_service_server::RefsService for RpcServiceCore {
    /// Lists references (branches and/or tags) for the specified repository filtered by the request flags.
    ///
    /// Given an RpcRefsRequest that includes a repository and boolean flags, returns the repository's refs
    /// filtered to include tags when `tag` is true and branches when `branch` is true. Each returned entry
    /// contains the repository, name, full_name, hash, type, and whether it is the default (head).
    ///
    /// # Returns
    ///
    /// A `RpcRefsResponse` whose `refs` field is a vector of `RpcRefs` matching the requested filters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assume `svc` implements the generated gRPC client and `req` is an RpcRefsRequest with repository set.
    /// // let resp = svc.refs(req).await.unwrap();
    /// // assert!(resp.get_ref().refs.len() >= 0);
    /// ```
    async fn refs(
        &self,
        request: Request<RpcRefsRequest>
    ) -> Result<Response<RpcRefsResponse>, Status> {
        let inner = request.into_inner();
        let Some(rpc_repo) = inner
            .repository else {
            return Err(Status::new(Code::Unavailable, "Repository not found"));
        };
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo.clone())
            .await
            .map_err(|e| Status::new(Code::Unavailable, format!("Repository not found: {:?}", e)))?;
        let refs = repo.refs
            .refs()
            .await
            .map_err(|e| Status::new(Code::Unavailable, format!("Refs error: {:?}", e)))?;
        let mut result = vec![];
        for item in refs {
            if inner.tag && item.is_tag {
                result.push(item);
            } else if inner.branch && item.is_branch {
                result.push(item);
            } else {
                continue
            }
        };
        let r  = result
            .iter()
            .map(|x|x.clone())
            .map(|x| {
                RpcRefs {
                    repository: Option::from(rpc_repo.clone()),
                    name: x.name.clone(),
                    full_name: x.name.to_string(),
                    hash: x.value.to_string(),
                    r#type: 0,
                    is_default: x.is_head,
                }
            })
            .collect::<Vec<RpcRefs>>();
        Ok(Response::new(RpcRefsResponse { refs: r }))
    }

    /// Exchanges the repository's default branch with the provided branch name and returns an empty refs response.
    ///
    /// On success, returns an `RpcRefsExchangeDefaultResponse` with its `refs` field set to `None`.
    ///
    /// Returns an error `Status` with `Code::Unavailable` if the request omits the repository, if the repository cannot be resolved, or if the refs exchange operation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # async fn example(service: crate::rpc::service::RpcServiceCore) {
    /// use tonic::Request;
    /// use crate::rpc::gitfs::RpcRefsExchangeDefaultRequest;
    ///
    /// let req = RpcRefsExchangeDefaultRequest {
    ///     repository: Some(/* RpcRepository */),
    ///     default_branch: "main".to_string(),
    /// };
    ///
    /// let response = service.refs_exchange_default(Request::new(req)).await.unwrap();
    /// assert!(response.get_ref().refs.is_none());
    /// # }
    /// ```
    async fn refs_exchange_default(
        &self,
        request: Request<RpcRefsExchangeDefaultRequest>
    ) -> Result<Response<RpcRefsExchangeDefaultResponse>, Status> {
        let inner = request.into_inner();
        let Some(rpc_repo) = inner
            .repository else {
            return Err(Status::new(Code::Unavailable, "Repository not found"));
        };
        let repo = rpc_repository_to_inner_repository(self.app.clone(), rpc_repo.clone())
            .await
            .map_err(|e| Status::new(Code::Unavailable, format!("Repository not found: {:?}", e)))?;
        repo
            .refs
            .exchange_default_branch(inner.default_branch)
            .await
            .map_err(|e| Status::new(Code::Unavailable, format!("Refs error: {:?}", e)))?;
        Ok(Response::new(RpcRefsExchangeDefaultResponse {
            refs: None,
        }))
    }
}