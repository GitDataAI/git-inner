use std::path::PathBuf;
use async_trait::async_trait;
use uuid::Uuid;
use std::fs;
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use bytes::Bytes;
use crate::error::GitInnerError;
use crate::odb::{Object, Odb, OdbTransaction};
use crate::sha::HashValue;
use crate::sha::Sha;

pub struct OdbLocalStore {
    pub uid: Uuid,
}

impl OdbLocalStore {
    pub(crate) fn new(p0: Uuid) -> Self {
        OdbLocalStore {
            uid: p0,
        }
    }
}

impl OdbLocalStore {
    pub fn path(&self) -> PathBuf {
        let path = PathBuf::from(format!("./data/{}/odb", self.uid.to_string()));
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Failed to create directory");
        }
        return path;
    }
    
    fn object_path(&self, object_id: &HashValue) -> PathBuf {
        let hex_string = format!("{}", object_id);
        let dir = &hex_string[0..2];
        let file = &hex_string[2..];
        self.path().join(dir).join(file)
    }
    
    fn ensure_object_dir(&self, object_id: &HashValue) -> Result<(), GitInnerError> {
        let dir_path = self.object_path(object_id).parent().unwrap().to_path_buf();
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        Ok(())
    }
}


#[async_trait]
impl Odb for OdbLocalStore {
    async fn get_object(&self, object_id: HashValue) -> Option<Object> {
        let path = self.object_path(&object_id);
        
        if !path.exists() {
            return None;
        }
        
        let file = fs::File::open(&path).ok()?;
        let mut decoder = ZlibDecoder::new(file);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data).ok()?;
        Some(Object::None)
    }

    async fn put_object(&self, object: Object) -> Result<HashValue, GitInnerError> {
        let data = match &object {
            Object::Tree(tree) => {
                // For now we just create empty data for Tree
                Bytes::from(format!("tree {:?}", tree.id))
            },
            Object::Commit(commit) => {
                // For now we just create empty data for Commit
                Bytes::from(format!("commit {:?}", commit.hash))
            },
            Object::Blob(blob) => blob.data.clone(),
            Object::Tag(tag) => {
                // For now we just create empty data for Tag
                Bytes::from(format!("tag {:?}", tag.id))
            },
            Object::None => return Err(GitInnerError::InvalidData),
        };
        
        let object_type = object.object_type();
        let header = format!("{} {}\0", object_type.to_str(), data.len());
        let mut content = Vec::new();
        content.extend_from_slice(header.as_bytes());
        content.extend_from_slice(&data);
        
        // Calculate hash
        let mut hash = HashValue::new(crate::sha::HashVersion::Sha1);
        hash.update(&content);
        hash.finalize();
        
        // Ensure directory exists
        self.ensure_object_dir(&hash)?;
        
        // Write compressed object
        let path = self.object_path(&hash);
        let file = fs::File::create(&path)
            .map_err(|_| GitInnerError::LockError)?;
        let mut encoder = ZlibEncoder::new(file, Compression::default());
        encoder.write_all(&content)
            .map_err(|_| GitInnerError::LockError)?;
        encoder.finish()
            .map_err(|_| GitInnerError::LockError)?;
            
        Ok(hash)
    }

    async fn exists(&self, object_id: HashValue) -> Result<bool, GitInnerError> {
        Ok(self.object_path(&object_id).exists())
    }

    async fn delete_object(&self, object_id: HashValue) -> Result<bool, GitInnerError> {
        let path = self.object_path(&object_id);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|_| GitInnerError::LockError)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_objects(&self) -> Result<Vec<HashValue>, GitInnerError> {
        let mut objects = Vec::new();
        let root_path = self.path();
        
        if root_path.exists() {
            for entry in fs::read_dir(&root_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                if entry.file_type().map_err(|_| GitInnerError::LockError)?.is_dir() {
                    let dir_name = entry.file_name();
                    let dir_name_str = dir_name.to_string_lossy();
                    
                    for file_entry in fs::read_dir(entry.path())
                        .map_err(|_| GitInnerError::LockError)? {
                        let file_entry = file_entry.map_err(|_| GitInnerError::LockError)?;
                        let file_name = file_entry.file_name();
                        let file_name_str = file_name.to_string_lossy();
                        
                        let hash_str = format!("{}{}", dir_name_str, file_name_str);
                        if let Some(hash) = HashValue::from_str(&hash_str) {
                            objects.push(hash);
                        }
                    }
                }
            }
        }
        
        Ok(objects)
    }

    async fn clear_repo(&self) -> Result<(), GitInnerError> {
        let path = self.path();
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        // Recreate the directory
        fs::create_dir_all(&path)
            .map_err(|_| GitInnerError::LockError)?;
        Ok(())
    }

    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError> {
        Ok(Box::new(OdbLocalStoreTransaction::new(self.uid)))
    }
}

pub struct OdbLocalStoreTransaction {
    pub uid: Uuid,
    pub time: u64,
}

impl OdbLocalStoreTransaction {
    pub fn new(uid: Uuid) -> Self {
        OdbLocalStoreTransaction {
            uid,
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    pub fn path(&self) -> PathBuf {
        let path = PathBuf::from(format!("./data/{}/transaction/{}", self.uid.to_string(),self.time));
        if !path.exists() {
            fs::create_dir_all(&path)
                .expect("Failed to create transaction directory");
        }
        return path;
    }
    
    fn object_path(&self, object_id: &HashValue) -> PathBuf {
        let hex_string = format!("{}", object_id);
        let dir = &hex_string[0..2];
        let file = &hex_string[2..];
        self.path().join(dir).join(file)
    }
    
    fn ensure_object_dir(&self, object_id: &HashValue) -> Result<(), GitInnerError> {
        let dir_path = self.object_path(object_id).parent().unwrap().to_path_buf();
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Odb for OdbLocalStoreTransaction {
    async fn get_object(&self, object_id: HashValue) -> Option<Object> {
        let path = self.object_path(&object_id);
        
        if !path.exists() {
            return None;
        }
        
        let file = fs::File::open(&path).ok()?;
        let mut decoder = ZlibDecoder::new(file);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data).ok()?;
        Some(Object::None)
    }

    async fn put_object(&self, object: Object) -> Result<HashValue, GitInnerError> {
        let data = match &object {
            Object::Tree(tree) => {
                // For now we just create empty data for Tree
                Bytes::from(format!("tree {:?}", tree.id))
            },
            Object::Commit(commit) => {
                // For now we just create empty data for Commit
                Bytes::from(format!("commit {:?}", commit.hash))
            },
            Object::Blob(blob) => blob.data.clone(),
            Object::Tag(tag) => {
                // For now we just create empty data for Tag
                Bytes::from(format!("tag {:?}", tag.id))
            },
            Object::None => return Err(GitInnerError::InvalidData),
        };
        
        let object_type = object.object_type();
        let header = format!("{} {}\0", object_type.to_str(), data.len());
        let mut content = Vec::new();
        content.extend_from_slice(header.as_bytes());
        content.extend_from_slice(&data);
        
        // Calculate hash
        let mut hash = HashValue::new(crate::sha::HashVersion::Sha1);
        hash.update(&content);
        hash.finalize();
        
        // Ensure directory exists
        self.ensure_object_dir(&hash)?;
        
        // Write compressed object
        let path = self.object_path(&hash);
        let file = fs::File::create(&path)
            .map_err(|_| GitInnerError::LockError)?;
        let mut encoder = ZlibEncoder::new(file, Compression::default());
        encoder.write_all(&content)
            .map_err(|_| GitInnerError::LockError)?;
        encoder.finish()
            .map_err(|_| GitInnerError::LockError)?;
            
        Ok(hash)
    }

    async fn exists(&self, object_id: HashValue) -> Result<bool, GitInnerError> {
        Ok(self.object_path(&object_id).exists())
    }

    async fn delete_object(&self, object_id: HashValue) -> Result<bool, GitInnerError> {
        let path = self.object_path(&object_id);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|_| GitInnerError::LockError)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_objects(&self) -> Result<Vec<HashValue>, GitInnerError> {
        let mut objects = Vec::new();
        let root_path = self.path();
        
        if root_path.exists() {
            for entry in fs::read_dir(&root_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                if entry.file_type().map_err(|_| GitInnerError::LockError)?.is_dir() {
                    let dir_name = entry.file_name();
                    let dir_name_str = dir_name.to_string_lossy();
                    
                    for file_entry in fs::read_dir(entry.path())
                        .map_err(|_| GitInnerError::LockError)? {
                        let file_entry = file_entry.map_err(|_| GitInnerError::LockError)?;
                        let file_name = file_entry.file_name();
                        let file_name_str = file_name.to_string_lossy();
                        
                        let hash_str = format!("{}{}", dir_name_str, file_name_str);
                        if let Some(hash) = HashValue::from_str(&hash_str) {
                            objects.push(hash);
                        }
                    }
                }
            }
        }
        
        Ok(objects)
    }

    async fn clear_repo(&self) -> Result<(), GitInnerError> {
        let path = self.path();
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        fs::create_dir_all(&path)
            .map_err(|_| GitInnerError::LockError)?;
        Ok(())
    }

    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError> {
        Ok(Box::new(self.clone()))
    }
}

#[async_trait]
impl OdbTransaction for OdbLocalStoreTransaction {
    async fn commit(&self) -> Result<(), GitInnerError> {
        let transaction_path = self.path();
        let store_path = PathBuf::from(format!("./data/{}.odb", self.uid.to_string()));
        if transaction_path.exists() {
            for entry in fs::read_dir(&transaction_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                if entry.file_type().map_err(|_| GitInnerError::LockError)?.is_dir() {
                    let dir_name = entry.file_name();
                    let source_dir = entry.path();
                    let target_dir = store_path.join(&dir_name);
                    if !target_dir.exists() {
                        fs::create_dir_all(&target_dir)
                            .map_err(|_| GitInnerError::LockError)?;
                    }
                    for file_entry in fs::read_dir(&source_dir)
                        .map_err(|_| GitInnerError::LockError)? {
                        let file_entry = file_entry.map_err(|_| GitInnerError::LockError)?;
                        let file_name = file_entry.file_name();
                        let source_file = file_entry.path();
                        let target_file = target_dir.join(&file_name);
                        if target_file.exists() {
                            fs::remove_file(&target_file)
                                .map_err(|_| GitInnerError::LockError)?;
                        }
                        fs::rename(&source_file, &target_file)
                            .map_err(|_| GitInnerError::LockError)?;
                    }
                }
            }
            fs::remove_dir_all(&transaction_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        
        Ok(())
    }

    async fn abort(&self) -> Result<(), GitInnerError> {
        let transaction_path = self.path();
        if transaction_path.exists() {
            fs::remove_dir_all(&transaction_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        Ok(())
    }

    async fn rollback(&self) -> Result<(), GitInnerError> {
        let transaction_path = self.path();
        if transaction_path.exists() {
            fs::remove_dir_all(&transaction_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        Ok(())
    }
}

impl Clone for OdbLocalStore {
    fn clone(&self) -> Self {
        OdbLocalStore {
            uid: self.uid,
        }
    }
}

impl Clone for OdbLocalStoreTransaction {
    fn clone(&self) -> Self {
        OdbLocalStoreTransaction {
            uid: self.uid,
            time: self.time,
        }
    }
}