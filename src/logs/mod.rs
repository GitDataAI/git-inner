use std::collections::BTreeMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use byteorder::{ByteOrder, LittleEndian};
use lru::LruCache;
use chrono::{DateTime, Utc};

const MAX_MEM_ENTRIES: usize = 100_000;
const MAX_DISK_BYTES: u64 = 500 * 1024 * 1024;
const MAX_RETENTION_DAYS: i64 = 7;

type Key = u64;
type Value = Vec<u8>;

#[derive(Debug)]
pub enum LogsError {
    IoError(std::io::Error),
    LockError(String),
    InvalidState(String),
}

impl From<std::io::Error> for LogsError {
    fn from(err: std::io::Error) -> Self {
        LogsError::IoError(err)
    }
}

impl std::fmt::Display for LogsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogsError::IoError(e) => write!(f, "IO error: {}", e),
            LogsError::LockError(msg) => write!(f, "Lock error: {}", msg),
            LogsError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl std::error::Error for LogsError {}

pub struct DiskMeta {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: SystemTime,
}

#[derive(Clone)]
pub struct LogsStore {
    mem: Arc<Mutex<LruCache<Key, Value>>>,
    dir: PathBuf,
    disk_files: Arc<Mutex<BTreeMap<SystemTime, DiskMeta>>>,
    current: Arc<Mutex<Option<BufWriter<File>>>>,
    current_size: Arc<Mutex<u64>>,
    current_ts: Arc<Mutex<SystemTime>>,
}

impl LogsStore {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, LogsError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        let mut map = BTreeMap::new();
        let mut total = 0u64;

        // 启动时扫描
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }
            let meta = entry.metadata()?;
            let len = meta.len();
            let mtime = meta.modified()?;
            total += len;
            map.insert(mtime, DiskMeta { path, size: len, mtime });
        }

        let store = LogsStore {
            mem: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(MAX_MEM_ENTRIES)
                    .ok_or_else(|| LogsError::InvalidState("Invalid MAX_MEM_ENTRIES".to_string()))?,
            ))),
            dir,
            disk_files: Arc::new(Mutex::new(map)),
            current: Arc::new(Mutex::new(None)),
            current_size: Arc::new(Mutex::new(0)),
            current_ts: Arc::new(Mutex::new(UNIX_EPOCH)),
        };

        store.evict_disk(total);
        Ok(store)
    }

    pub fn put(&self, key: Key, value: Value) -> Result<(), LogsError> {
        let mut mem = self.mem.lock()
            .map_err(|e| LogsError::LockError(format!("Failed to lock mem: {}", e)))?;

        if let Some((_, evicted)) = mem.push(key, value) {
            self.append_to_disk(&evicted)?;
        }
        Ok(())
    }

    fn append_to_disk(&self, data: &[u8]) -> Result<(), LogsError> {
        let now = SystemTime::now();

        {
            let mut curr_ts = self.current_ts.lock()
                .map_err(|e| LogsError::LockError(format!("Failed to lock current_ts: {}", e)))?;
            let duration_since = now.duration_since(*curr_ts).unwrap_or_default();
            if duration_since >= Duration::from_secs(60) {
                *curr_ts = now;
                drop(curr_ts);
                self.rotate_file(now)?;
            }
        }

        let mut writer = self.current.lock()
            .map_err(|e| LogsError::LockError(format!("Failed to lock writer: {}", e)))?;
        let mut size = self.current_size.lock()
            .map_err(|e| LogsError::LockError(format!("Failed to lock size: {}", e)))?;

        let w = writer.as_mut()
            .ok_or_else(|| LogsError::InvalidState("No current writer available".to_string()))?;

        // 格式：timestamp(8) + len(4) + payload
        let mut header = [0u8; 12];
        let ts = now.duration_since(UNIX_EPOCH)
            .map_err(|e| LogsError::InvalidState(format!("Invalid timestamp: {}", e)))?
            .as_secs();

        LittleEndian::write_u64(&mut header[0..8], ts);
        LittleEndian::write_u32(&mut header[8..12], data.len() as u32);
        w.write_all(&header)?;
        w.write_all(data)?;
        w.flush()?;
        *size += 12 + data.len() as u64;

        Ok(())
    }

    /// 滚动新文件
    fn rotate_file(&self, now: SystemTime) -> Result<(), LogsError> {
        // 先关闭旧文件
        {
            let mut current = self.current.lock()
                .map_err(|e| LogsError::LockError(format!("Failed to lock current: {}", e)))?;
            *current = None;
        }

        let new_name = {
            let dt: DateTime<Utc> = now.into();
            format!("metrics.{}.log", dt.format("%Y%m%d-%H%M"))
        };
        let path = self.dir.join(new_name);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        {
            let mut current = self.current.lock()
                .map_err(|e| LogsError::LockError(format!("Failed to lock current: {}", e)))?;
            *current = Some(BufWriter::new(file));
        }

        {
            let mut size = self.current_size.lock()
                .map_err(|e| LogsError::LockError(format!("Failed to lock size: {}", e)))?;
            *size = 0;
        }

        // 把新文件加入索引
        let meta = fs::metadata(&path)?;
        let disk_meta = DiskMeta {
            path: path.clone(),
            size: meta.len(),
            mtime: now,
        };

        {
            let mut disk_files = self.disk_files.lock()
                .map_err(|e| LogsError::LockError(format!("Failed to lock disk_files: {}", e)))?;
            disk_files.insert(now, disk_meta);
        }

        // 检查磁盘驱逐
        self.evict_disk(0); // 0 表示先不计算新文件大小，后面再删

        Ok(())
    }

    /// 磁盘 LRU 驱逐
    fn evict_disk(&self, additional: u64) {
        let mut files = match self.disk_files.lock() {
            Ok(files) => files,
            Err(e) => {
                eprintln!("Failed to lock disk_files for eviction: {}", e);
                return;
            }
        };

        let mut total: u64 = files.values().map(|m| m.size).sum::<u64>() + additional;
        let cutoff = SystemTime::now() - Duration::from_secs(MAX_RETENTION_DAYS as u64 * 86400);

        while let Some((&oldest_time, meta)) = files.iter().next() {
            let need_evict = total > MAX_DISK_BYTES || oldest_time < cutoff;
            if !need_evict {
                break;
            }

            // 删除文件
            if let Err(e) = fs::remove_file(&meta.path) {
                eprintln!("Failed to remove {:?}: {}", meta.path, e);
            } else {
                total = total.saturating_sub(meta.size);
            }

            files.pop_first();
        }
    }
}
