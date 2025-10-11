use tonic::{Code, Request, Response, Status};
use crate::error::GitInnerError;
use crate::rpc::gitfs::{RpcRefs, RpcRefsExchangeDefaultRequest, RpcRefsExchangeDefaultResponse, RpcRefsRequest, RpcRefsResponse};
use crate::rpc::rpc_repository_to_inner_repository;
use crate::serve::AppCore;

pub struct RefsService {
    app: AppCore,
}

impl RefsService {
    pub fn init() -> Result<Self, GitInnerError>{
        let app = AppCore::app()?;
        Ok(RefsService { app } )
    }
}

#[tonic::async_trait]
impl crate::rpc::gitfs::refs_service_server::RefsService for RefsService {
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

    async fn refs_exchange_default(
        &self,
        request: Request<RpcRefsExchangeDefaultRequest>
    ) -> Result<Response<RpcRefsExchangeDefaultResponse>, Status> {
        todo!()
    }
}