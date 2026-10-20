//! Cover images on disk (spec 12.4), limited in size: when the total goes
//! over the limit, the least recently used covers are deleted.
//!
//! All files sit in one folder, named by a hash (never by text from the
//! server). The last use of each file is its modification time, so it
//! survives restarts; an index in memory is built from the folder on first use.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

/// After going over the limit, delete down to this share of it, so the next
/// covers do not each trigger a cleanup.
const CLEANUP_TARGET_PERCENT: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    size: u64,
    last_used: SystemTime,
}

#[derive(Debug, Default)]
struct Index {
    entries: HashMap<String, Entry>,
    total: u64,
}

impl Index {
    fn insert(&mut self, key: String, entry: Entry) {
        if let Some(old) = self.entries.insert(key, entry) {
            self.total -= old.size;
        }
        self.total += entry.size;
    }

    fn remove(&mut self, key: &str) {
        if let Some(old) = self.entries.remove(key) {
            self.total -= old.size;
        }
    }
}

pub struct CoverCache {
    dir: PathBuf,
    /// Bytes; changed from the settings (spec 6.10).
    limit: AtomicU64,
    /// `None` until the folder is scanned.
    index: Mutex<Option<Index>>,
}

impl CoverCache {
    pub fn new(dir: PathBuf, limit: u64) -> Self {
        Self { dir, limit: AtomicU64::new(limit), index: Mutex::new(None) }
    }

    /// A new limit; a smaller one deletes the least recently used covers now.
    pub fn set_limit(&self, limit: u64) {
        self.limit.store(limit, Ordering::Relaxed);
        let mut guard = self.index();
        if let Some(index) = guard.as_mut() {
            self.evict(index, limit);
        }
    }

    /// Bytes on disk.
    pub fn size(&self) -> u64 {
        self.index().as_ref().map_or(0, |index| index.total)
    }

    /// Deletes every cover ("Clear cache"); they are downloaded again when shown.
    pub fn clear(&self) -> io::Result<()> {
        let mut guard = self.index();
        if let Some(index) = guard.as_mut() {
            let keys: Vec<String> = index.entries.keys().cloned().collect();
            for key in keys {
                match fs::remove_file(self.dir.join(&key)) {
                    Ok(()) => index.remove(&key),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => index.remove(&key),
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    /// The cached image, if any. Marks it as just used.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.index().as_ref()?.entries.contains_key(key) {
            return None;
        }
        // Read outside the lock, so covers load in parallel.
        let path = self.dir.join(key);
        let bytes = fs::read(&path);
        let mut index = self.index();
        let index = index.as_mut()?;
        match bytes {
            Ok(bytes) => {
                let now = SystemTime::now();
                if let Some(entry) = index.entries.get_mut(key) {
                    entry.last_used = now;
                }
                // Best effort: the date only orders the cleanup.
                let _ = fs::File::options().write(true).open(&path).and_then(|f| f.set_modified(now));
                Some(bytes)
            }
            Err(_) => {
                index.remove(key);
                None
            }
        }
    }

    /// Stores an image, then deletes the least recently used ones if the
    /// cache went over its limit.
    pub fn put(&self, key: &str, bytes: &[u8]) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(key);
        let temp = self.dir.join(format!("{key}.tmp"));
        fs::write(&temp, bytes)?;
        fs::rename(&temp, &path)?;

        let mut guard = self.index();
        let Some(index) = guard.as_mut() else { return Ok(()) };
        index.insert(key.to_string(), Entry { size: bytes.len() as u64, last_used: SystemTime::now() });
        self.evict(index, self.limit.load(Ordering::Relaxed));
        Ok(())
    }

    fn evict(&self, index: &mut Index, limit: u64) {
        for victim in pick_evictions(&index.entries, index.total, limit) {
            // A file that cannot be deleted now is found again by the next scan.
            let _ = fs::remove_file(self.dir.join(&victim));
            index.remove(&victim);
        }
    }

    /// The index, scanning the folder the first time.
    fn index(&self) -> MutexGuard<'_, Option<Index>> {
        let mut guard = self.index.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            *guard = Some(self.scan());
        }
        guard
    }

    fn scan(&self) -> Index {
        let mut index = Index::default();
        let Ok(files) = fs::read_dir(&self.dir) else { return index };
        for file in files.flatten() {
            let path = file.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
                continue;
            };
            // Left over by a write that did not finish.
            if name.ends_with(".tmp") {
                let _ = fs::remove_file(&path);
                continue;
            }
            let Ok(meta) = file.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let last_used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            index.insert(name, Entry { size: meta.len(), last_used });
        }
        index
    }
}

/// Keys to delete, least recently used first, when `total` is over `limit`.
fn pick_evictions(entries: &HashMap<String, Entry>, total: u64, limit: u64) -> Vec<String> {
    if total <= limit {
        return Vec::new();
    }
    let target = limit / 100 * CLEANUP_TARGET_PERCENT;
    let mut by_use: Vec<(&String, &Entry)> = entries.iter().collect();
    by_use.sort_by_key(|(key, entry)| (entry.last_used, key.as_str()));

    let mut remaining = total;
    let mut victims = Vec::new();
    for (key, entry) in by_use {
        if remaining <= target {
            break;
        }
        remaining -= entry.size;
        victims.push(key.clone());
    }
    victims
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn entries(list: &[(&str, u64, u64)]) -> HashMap<String, Entry> {
        list.iter()
            .map(|&(key, size, used)| (key.to_string(), Entry { size, last_used: at(used) }))
            .collect()
    }

    #[test]
    fn nothing_is_deleted_under_the_limit() {
        let list = entries(&[("a", 40, 1), ("b", 60, 2)]);
        assert!(pick_evictions(&list, 100, 100).is_empty());
    }

    #[test]
    fn the_least_recently_used_go_first_down_to_ninety_percent() {
        // 130 bytes for a 100-byte limit: delete down to 90.
        let list = entries(&[("new", 30, 50), ("old", 30, 10), ("older", 30, 5), ("mid", 40, 20)]);
        assert_eq!(pick_evictions(&list, 130, 100), ["older", "old"]);
    }

    #[test]
    fn cache_on_disk_stores_reads_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("parlo-covers-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = CoverCache::new(dir.clone(), 100);

        assert_eq!(cache.get("a"), None);
        cache.put("a", &[1; 40]).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        cache.put("b", &[2; 40]).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        // "a" is used again, so "b" becomes the oldest.
        assert_eq!(cache.get("a").as_deref(), Some(&[1u8; 40][..]));
        std::thread::sleep(Duration::from_millis(20));
        cache.put("c", &[3; 40]).unwrap();

        assert_eq!(cache.get("b"), None, "least recently used is deleted");
        assert!(cache.get("a").is_some() && cache.get("c").is_some());
        assert!(!dir.join("b").exists());

        // A new cache over the same folder finds what is there.
        let reopened = CoverCache::new(dir.clone(), 100);
        assert!(reopened.get("c").is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_limit_changes_and_the_cache_clears() {
        let dir = std::env::temp_dir().join(format!("parlo-covers-limit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = CoverCache::new(dir.clone(), 1000);
        for (index, key) in ["a", "b", "c"].iter().enumerate() {
            cache.put(key, &[1; 100]).unwrap();
            std::thread::sleep(Duration::from_millis(20 * (index as u64 + 1)));
        }
        assert_eq!(cache.size(), 300);

        // A smaller limit deletes the oldest at once: down to 90% of 200.
        cache.set_limit(200);
        assert_eq!(cache.size(), 100);
        assert!(cache.get("c").is_some() && cache.get("a").is_none());

        cache.clear().unwrap();
        assert_eq!(cache.size(), 0);
        assert!(!dir.join("c").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
