use crate::serve::AppCore;

pub mod refs;
pub mod repository;
pub mod commit;
pub mod tree;


#[derive(Clone)]
pub struct RpcServiceCore {
    pub app: AppCore,
}