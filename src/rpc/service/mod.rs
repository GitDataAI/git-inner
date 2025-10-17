use crate::serve::AppCore;

pub mod commit;
pub mod refs;
pub mod repository;
pub mod tree;

#[derive(Clone)]
pub struct RpcServiceCore {
    pub app: AppCore,
}
