use crate::error::GitInnerError;
use crate::odb::mongo::odb::OdbMongoObject;
use crate::refs::mongo::MongoRefsManager;
use crate::repository::Repository;
use crate::sha::HashVersion;
use async_trait::async_trait;
use mongodb::bson::{Uuid, doc};
use mongodb::{Client, Collection};
use object_store::ObjectStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use object_store::local::LocalFileSystem;
use crate::serve::{AppCore, RepoStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MongoRepository {
    pub id: i32,
    pub name: String,
    pub namespace: String,
    pub uid: Uuid,
    pub hash_version: i32,
    pub default_branch: String
}

#[derive(Debug, Clone)]
pub struct MongoRepoManager {
    pub db_client: Client,
    pub repo: Collection<MongoRepository>,
    pub store: Arc<Box<dyn ObjectStore>>,
}

impl MongoRepoManager {
    pub fn new(db_client: Client, store: Arc<Box<dyn ObjectStore>>) -> Self {
        let db = db_client.database("git_inner");
        let repo = db.collection::<MongoRepository>("repositories");
        MongoRepoManager {
            db_client,
            repo,
            store,
        }
    }
}

pub async fn init_app_by_mongodb() {
    dotenv::dotenv().ok();
    let mongodb_url = dotenv::var("MONGODB_URL").expect("MONGODB_URL must be set");
    let store = LocalFileSystem::new_with_prefix("./data")
        .expect("Failed to initialize local storage");
    let optional = mongodb::options::ClientOptions::parse(mongodb_url)
        .await
        .expect("Failed to parse MongoDB client options");
    let mongodb = mongodb::Client::with_options(optional)
        .expect("Failed to create MongoDB client");
    let manager = MongoRepoManager::new(mongodb, Arc::new(Box::new(store)));
    let core = AppCore::new(Arc::new(Box::new(manager)));
    let _ = core.init();
    
}

#[async_trait]
impl RepoStore for MongoRepoManager {
    async fn repo(&self, namespace: String, name: String) -> Result<Repository, GitInnerError> {
        let mongo_repo = self
            .repo
            .find_one(doc! {
                "namespace": &namespace,
                "name": &name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?
            .ok_or_else(|| GitInnerError::ObjectNotFound(HashVersion::Sha1.default()))?;
        let hash_version = match mongo_repo.hash_version {
            1 => HashVersion::Sha1,
            256 => HashVersion::Sha256,
            _ => return Err(GitInnerError::HashVersionError),
        };
        let db_name = "git_inner";
        let db = self.db_client.database(db_name);
        let odb = OdbMongoObject {
            repo_uid: mongo_repo.uid.clone(),
            store: self.store.clone(),
            db_client: self.db_client.clone(),
            commit: db.collection("commits"),
            tag: db.collection("tags"),
            tree: db.collection("trees"),
        };
        let refs = MongoRefsManager {
            repo_uid: mongo_repo.uid.clone(),
            default_branch: mongo_repo.default_branch.clone(),
            db_client: self.db_client.clone(),
            refs: db.collection("refs"),
            hash_version: hash_version.clone(),
        };
        Ok(Repository {
            id: uuid::Uuid::from_slice(mongo_repo.uid.bytes().as_slice())
                .map_err(|_| GitInnerError::UuidError)?,
            default_branch: mongo_repo.default_branch,
            odb: Arc::new(Box::new(odb)),
            refs: Arc::new(Box::new(refs)),
            hash_version,
        })
    }
}
