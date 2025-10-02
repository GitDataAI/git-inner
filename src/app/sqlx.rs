use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use crate::app::RepoStore;
use crate::error::GitInnerError;
use crate::odb::localstore::OdbLocalStore;
use crate::refs::localstore::RefLocalStore;
use crate::repository::Repository;
use crate::sha::HashVersion;

pub struct SqliteRepository {
    pub id: i32,
    pub name: String,
    pub namespace: String,
    pub uid: Uuid,
    pub hash_version: i32,
}

impl SqliteRepository {
    pub fn new(id: i32, name: String, namespace: String, uid: Uuid, re: i32) -> Self {
        SqliteRepository {
            id,
            name,
            namespace,
            uid,
            hash_version: re,
        }
    }
}


#[derive(Clone)]
pub struct SqliteConn {
    pub conn: Arc<rusqlite::Connection>,
}


impl SqliteConn {
    pub fn new(conn: rusqlite::Connection) -> Self {
        SqliteConn {
            conn: Arc::new(conn),
        }
    }
}

unsafe impl Send for SqliteConn {}
unsafe impl Sync for SqliteConn {}

impl SqliteConn {
    pub fn init_table(&self) -> Result<(), GitInnerError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                namespace TEXT NOT NULL,
                uid TEXT NOT NULL,
                version INTEGER NOT NULL
            )",
            (),
        )
            .map_err(|e| GitInnerError::SqliteError(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl RepoStore for SqliteConn {
    async fn repo(&self, namespace: String, name: String) -> Result<Repository, GitInnerError> {
        let repo = self.conn.query_row(
            "SELECT id, name, namespace, uid, version FROM repositories WHERE namespace = ? AND name = ?",
            (namespace, name),
            |row| {
                let uid_str: String = row.get(3)?;
                let uid = Uuid::parse_str(&uid_str)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;
                
                Ok(SqliteRepository::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    uid,
                    row.get(4)?
                ))
            },
        )
            .map_err(|e| GitInnerError::SqliteError(e.to_string()))?;
        let uid = repo.uid;
        let odb = OdbLocalStore::new(uid);
        let refs = RefLocalStore::new(uid);
        Ok(
            Repository {
                id: uid,
                odb: Box::new(odb),
                refs: Box::new(refs),
                hash_version: match repo.hash_version {
                    1 => HashVersion::Sha1,
                    256 => HashVersion::Sha256,
                    _ => return Err(GitInnerError::HashVersionError),
                }
            }
        )
    }
}