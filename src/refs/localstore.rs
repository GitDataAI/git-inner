use std::path::PathBuf;
use async_trait::async_trait;
use uuid::Uuid;
use std::fs;
use std::io::Read;
use crate::error::GitInnerError;
use crate::refs::{RefItem, RefsManager};
use crate::sha::HashValue;

pub struct RefLocalStore {
    pub uid: Uuid,
}

impl RefLocalStore {
    pub fn new(uid: Uuid) -> Self {
        RefLocalStore {
            uid,
        }
    }
    
    pub fn path(&self) -> PathBuf {
        let path = PathBuf::from(format!("./data/{}/refs", self.uid.to_string()));
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Failed to create refs directory");
        }
         path
    }
    
    fn ref_path(&self, ref_name: &str) -> PathBuf {
        self.path().join(ref_name)
    }
}


#[async_trait]
impl RefsManager for RefLocalStore {
    async fn head(&self) -> Result<RefItem, GitInnerError> {
        let head_path = self.path().join("HEAD");
        if head_path.exists() {
            let content = fs::read_to_string(&head_path)
                .map_err(|_| GitInnerError::LockError)?;
            
            let parts: Vec<&str> = content.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(hash) = HashValue::from_str(parts[1]) {
                    return Ok(RefItem {
                        name: "HEAD".to_string(),
                        value: hash,
                        is_branch: false,
                        is_tag: false,
                        is_head: true,
                    });
                }
            }
        }
        
        // Return a default HEAD if not found
        Ok(RefItem {
            name: "HEAD".to_string(),
            value: HashValue::new(crate::sha::HashVersion::Sha1),
            is_branch: false,
            is_tag: false,
            is_head: true,
        })
    }

    async fn refs(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let mut refs = Vec::new();
        let refs_path = self.path();
        
        if refs_path.exists() {
            for entry in fs::read_dir(&refs_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                
                // Skip HEAD as it's handled separately
                if file_name_str == "HEAD" {
                    continue;
                }
                
                let file_path = entry.path();
                if file_path.is_file() {
                    let mut file = fs::File::open(&file_path)
                        .map_err(|_| GitInnerError::LockError)?;
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|_| GitInnerError::LockError)?;
                    
                    if let Some(hash) = HashValue::from_str(content.trim()) {
                        let is_tag = file_name_str.starts_with("tags/");
                        let is_branch = !is_tag;
                        
                        refs.push(RefItem {
                            name: file_name_str.to_string(),
                            value: hash,
                            is_branch,
                            is_tag,
                            is_head: false,
                        });
                    }
                }
            }
        }
        
        Ok(refs)
    }

    async fn tags(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let mut tags = Vec::new();
        let tags_path = self.path().join("tags");
        
        if tags_path.exists() {
            for entry in fs::read_dir(&tags_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                
                let file_path = entry.path();
                if file_path.is_file() {
                    let mut file = fs::File::open(&file_path)
                        .map_err(|_| GitInnerError::LockError)?;
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|_| GitInnerError::LockError)?;
                    
                    if let Some(hash) = HashValue::from_str(content.trim()) {
                        tags.push(RefItem {
                            name: format!("tags/{}", file_name_str),
                            value: hash,
                            is_branch: false,
                            is_tag: true,
                            is_head: false,
                        });
                    }
                }
            }
        }
        
        Ok(tags)
    }

    async fn branches(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let mut branches = Vec::new();
        let refs_path = self.path();
        
        if refs_path.exists() {
            for entry in fs::read_dir(&refs_path)
                .map_err(|_| GitInnerError::LockError)? {
                let entry = entry.map_err(|_| GitInnerError::LockError)?;
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                
                // Skip HEAD and tags directory
                if file_name_str == "HEAD" || file_name_str == "tags" {
                    continue;
                }
                
                let file_path = entry.path();
                if file_path.is_file() {
                    let mut file = fs::File::open(&file_path)
                        .map_err(|_| GitInnerError::LockError)?;
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|_| GitInnerError::LockError)?;
                    
                    if let Some(hash) = HashValue::from_str(content.trim()) {
                        branches.push(RefItem {
                            name: file_name_str.to_string(),
                            value: hash,
                            is_branch: true,
                            is_tag: false,
                            is_head: false,
                        });
                    }
                }
            }
        }
        
        Ok(branches)
    }

    async fn del_refs(&self, ref_name: String) -> Result<(), GitInnerError> {
        let ref_path = self.ref_path(&ref_name);
        if ref_path.exists() {
            fs::remove_file(&ref_path)
                .map_err(|_| GitInnerError::LockError)?;
        }
        Ok(())
    }

    async fn create_refs(&self, ref_name: String, ref_value: HashValue) -> Result<(), GitInnerError> {
        let ref_path = self.ref_path(&ref_name);
        // 确保父目录存在，但不递归创建
        if let Some(parent) = ref_path.parent() {
            if !parent.exists() {
                return Err(GitInnerError::LockError);
            }
        }
        
        fs::write(&ref_path, ref_value.to_string())
            .map_err(|_| GitInnerError::LockError)?;
        Ok(())
    }

    async fn update_refs(&self, ref_name: String, ref_value: HashValue) -> Result<(), GitInnerError> {
        let ref_path = self.ref_path(&ref_name);
        // 确保父目录存在，但不递归创建
        if let Some(parent) = ref_path.parent() {
            if !parent.exists() {
                return Err(GitInnerError::LockError);
            }
        }
        
        fs::write(&ref_path, ref_value.to_string())
            .map_err(|_| GitInnerError::LockError)?;
        Ok(())
    }

    async fn get_refs(&self, ref_name: String) -> Result<RefItem, GitInnerError> {
        let ref_path = self.ref_path(&ref_name);
        if !ref_path.exists() {
            return Err(GitInnerError::LockError);
        }
        
        let content = fs::read_to_string(&ref_path)
            .map_err(|_| GitInnerError::LockError)?;
        
        if let Some(hash) = HashValue::from_str(content.trim()) {
            let is_tag = ref_name.starts_with("tags/");
            let is_branch = !is_tag && ref_name != "HEAD";
            let is_head = ref_name == "HEAD";
            
            Ok(RefItem {
                name: ref_name,
                value: hash,
                is_branch,
                is_tag,
                is_head,
            })
        } else {
            Err(GitInnerError::InvalidSha1String)
        }
    }

    async fn exists_refs(&self, ref_name: String) -> Result<bool, GitInnerError> {
        Ok(self.ref_path(&ref_name).exists())
    }

    async fn get_value_refs(&self, ref_name: String) -> Result<HashValue, GitInnerError> {
        let ref_path = self.ref_path(&ref_name);
        if !ref_path.exists() {
            return Err(GitInnerError::LockError);
        }
        
        let content = fs::read_to_string(&ref_path)
            .map_err(|_| GitInnerError::LockError)?;
        
        if let Some(hash) = HashValue::from_str(content.trim()) {
            Ok(hash)
        } else {
            Err(GitInnerError::InvalidSha1String)
        }
    }
}