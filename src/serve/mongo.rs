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
use crate::model::repository::MongoRepository;
use crate::rpc::gitfs::{RepositoryInitResponse, RpcRepository};
use crate::serve::{AppCore, RepoStore};


#[derive(Debug, Clone)]
pub struct MongoRepoManager {
    pub db_client: Client,
    pub repo: Collection<MongoRepository>,
    pub store: Arc<Box<dyn ObjectStore>>,
}

impl MongoRepoManager {
    /// Creates a new MongoRepoManager bound to the "git_inner" database and the "repositories" collection.
    ///
    /// The returned manager holds the provided MongoDB client and a shared object store for repository objects.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use mongodb::Client;
    /// // assume `store` implements ObjectStore and is already constructed
    /// let client = Client::with_uri_str("mongodb://localhost:27017").unwrap();
    /// let store: Arc<Box<dyn ObjectStore>> = /* construct store */ unimplemented!();
    /// let manager = MongoRepoManager::new(client, store);
    /// ```
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

/// Initializes application components using MongoDB for metadata and a local filesystem for object storage.
///
/// This sets up environment loading, constructs a local file-backed object store at "./data",
/// parses `MONGODB_URL` for a MongoDB client, creates a `MongoRepoManager` backed by that client
/// and the object store, builds an `AppCore` with the manager, and runs its initialization routine.
///
/// # Examples
///
/// ```
/// # // Example requires the Tokio runtime and a valid MONGODB_URL environment variable.
/// # // Run with: MONGODB_URL="mongodb://localhost:27017" cargo run --example init
/// #[tokio::main]
/// async fn main() {
///     init_app_by_mongodb().await;
/// }
/// ```
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
    let core = AppCore::new(Arc::new(Box::new(manager)), None);
    let _ = core.init();
    
}

#[async_trait]
impl RepoStore for MongoRepoManager {
    /// Constructs a Repository from MongoDB metadata and the shared object store.
    ///
    /// Returns a Repository assembled from the MongoDB document matching `namespace` and `name`.
    ///
    /// # Errors
    ///
    /// - `GitInnerError::MongodbError` if the MongoDB query fails.
    /// - `GitInnerError::ObjectNotFound(HashVersion::Sha1.default())` if no matching document is found.
    /// - `GitInnerError::HashVersionError` if the stored `hash_version` is unsupported.
    /// - `GitInnerError::UuidError` if the repository UID cannot be converted to a UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # async fn _example(manager: &crate::serve::mongo::MongoRepoManager) -> Result<(), crate::error::GitInnerError> {
    /// let repo = manager.repo("my_namespace".to_string(), "my_repo".to_string()).await?;
    /// assert!(!repo.default_branch.is_empty());
    /// # Ok(())
    /// # }
    /// ```
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
            owner: Default::default(),
            odb: Arc::new(Box::new(odb)),
            refs: Arc::new(Box::new(refs)),
            hash_version,
            is_public: mongo_repo.is_public,
        })
    }

    /// Creates a new repository record in the MongoDB `repositories` collection and returns initialization metadata.
    ///
    /// On success returns a `RepositoryInitResponse` containing the numeric id, repository uid, name, namespace,
    /// and `is_private` flag derived from `is_public`. If a MongoDB operation fails, returns `GitInnerError::MongodbError`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example usage (pseudo-code; replace `manager` with a real MongoRepoManager instance)
    /// # async fn example(manager: &crate::MongoRepoManager) {
    /// let resp = manager
    ///     .create_repo(
    ///         "namespace".to_string(),
    ///         "name".to_string(),
    ///         uuid::Uuid::new_v4(),
    ///         1,
    ///         uuid::Uuid::new_v4(),
    ///         "main".to_string(),
    ///         true,
    ///     )
    ///     .await;
    /// assert!(resp.is_ok());
    /// # }
    /// ```
    async fn create_repo(&self, namespace: String, name: String, owner: uuid::Uuid, hash_version: i32, uid: uuid::Uuid, default_branch: String, is_public: bool) -> Result<RepositoryInitResponse, GitInnerError> {
        let count = self
            .repo
            .count_documents(doc! {})
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        let mongo_repo = MongoRepository {
            id: (count + 1) as i32,
            namespace: namespace.clone(),
            name: name.clone(),
            owner: Uuid::from_bytes(owner.into_bytes().into()),
            hash_version,
            uid: Uuid::from_bytes(uid.into_bytes().into()),
            default_branch,
            is_public,
        };
        self.repo
            .insert_one(mongo_repo)
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        Ok(RepositoryInitResponse {
            id: (count + 1) as i64,
            uid: uid.to_string(),
            name,
            namespace,
            is_private: !is_public,
        })
    }
    /// Sets the repository's visibility flag (`is_public`) for the repository identified by `namespace` and `name`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `Err(GitInnerError::MongodbError(_))` if the update fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(manager: &crate::MongoRepoManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.set_visibility("my-namespace".into(), "my-repo".into(), true).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn set_visibility(&self, namespace: String, name: String, is_public: bool) -> Result<(), GitInnerError> {
        self.repo
            .update_one(
                doc! {
                    "namespace": &namespace,
                    "name": &name
                },
                doc! {
                    "$set": {
                        "is_public": is_public
                    }
                },
            )
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        Ok(())
    }
    /// Retrieve RPC-friendly metadata for a repository identified by `namespace` and `name`.
    ///
    /// On success returns an `RpcRepository` populated from the stored MongoDB document.
    /// If the database query fails, a `GitInnerError::MongodbError` is returned.
    /// If no matching repository is found, a `GitInnerError::ObjectNotFound` with the default SHA-1 hash version is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// // Async context required (e.g., inside an async test or function).
    /// // let info = manager.repo_info("my_ns".into(), "my_repo".into()).await.unwrap();
    /// // assert_eq!(info.name, "my_repo");
    /// ```
    async fn repo_info(&self, namespace: String, name: String) -> Result<RpcRepository, GitInnerError> {
        let mongo_repo = self
            .repo
            .find_one(doc! {
                "namespace": &namespace,
                "name": &name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?
            .ok_or_else(|| GitInnerError::ObjectNotFound(HashVersion::Sha1.default()))?;
            Ok(RpcRepository {
                id: mongo_repo.id as i64,
                uid: mongo_repo.uid.to_string(),
                owner: mongo_repo.owner.to_string(),
                name: mongo_repo.name,
                namespace: mongo_repo.namespace,
                is_private: !mongo_repo.is_public,
            })
    }
}