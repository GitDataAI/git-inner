use tonic::{Request, Response, Status};
use crate::rpc::gitfs::{RepositoryInfoRequest, RepositoryInitRequest, RepositoryInitResponse, RpcRepository};
use crate::rpc::service::RpcServiceCore;
use crate::serve::AppCore;


#[async_trait::async_trait]
impl crate::rpc::gitfs::repository_service_server::RepositoryService for RpcServiceCore {
    /// Initializes a repository from the provided request.
    ///
    /// Performs repository creation/initialization and returns the resulting initialization data on success.
    ///
    /// # Returns
    ///
    /// `Response<RepositoryInitResponse>` containing the initialization result on success; returns a gRPC `Status::internal` on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // let svc: RpcServiceCore = /* obtain service */ ;
    /// // let req = tonic::Request::new(RepositoryInitRequest { /* fields */ });
    /// // let resp = svc.init(req).await.unwrap().into_inner();
    /// ```
    async fn init(&self, request: Request<RepositoryInitRequest>) -> Result<Response<RepositoryInitResponse>, Status> {
        self.app.init_repository(request.into_inner()).await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    /// Marks the specified repository as public and returns the updated repository.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::RpcRepository;
    /// # async fn example(svc: &impl crate::rpc::gitfs::repository_service_server::RepositoryService) {
    /// let repo = RpcRepository { /* fields */ };
    /// let resp = svc.set_public(Request::new(repo)).await.unwrap();
    /// let updated: &RpcRepository = resp.get_ref();
    /// # }
    /// ```
    async fn set_public(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .set_public(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    /// Sets a repository's visibility to private and returns the updated repository.
    ///
    /// # Returns
    ///
    /// `RpcRepository` with visibility changed to private.
    ///
    /// # Examples
    ///
    /// ```
    /// // Acquire a service instance and repository request appropriate for your context.
    /// // let svc: RpcServiceCore = ...;
    /// // let req = tonic::Request::new(RpcRepository { name: "repo".into(), ..Default::default() });
    /// // let resp = futures::executor::block_on(svc.set_private(req)).unwrap();
    /// // assert_eq!(resp.get_ref().name, "repo");
    /// ```
    async fn set_private(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .set_private(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    /// Deletes the repository identified by the given `RpcRepository` and returns its metadata.
    ///
    /// # Returns
    ///
    /// The `RpcRepository` representing the deleted repository.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assume `svc` is an instance of RpcServiceCore and `repo` is an RpcRepository.
    /// // let svc: RpcServiceCore = ...;
    /// // let req = tonic::Request::new(repo);
    /// // let resp = tokio::runtime::Runtime::new().unwrap().block_on(svc.delete(req)).unwrap().into_inner();
    /// // assert_eq!(resp.id, /* expected id */);
    /// ```
    async fn delete(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }

    /// Retrieves repository information for the provided request.
    ///
    /// On success returns the repository data wrapped in a gRPC response. On failure
    /// returns a gRPC internal `Status` describing the error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use tonic::Request;
    /// # use crate::rpc::gitfs::{RepositoryInfoRequest, RpcRepository};
    /// # async fn example(svc: &super::RpcServiceCore) -> Result<(), tonic::Status> {
    /// let req = RepositoryInfoRequest { /* fill fields */ };
    /// let response = svc.info(Request::new(req)).await?;
    /// let repo: RpcRepository = response.into_inner();
    /// # Ok(())
    /// # }
    /// ```
    async fn info(&self, request: Request<RepositoryInfoRequest>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .repo_info(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }
}