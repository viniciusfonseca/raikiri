use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct Cache<K, V>
    where K : std::cmp::Eq + std::hash::Hash {
    pub hashes: Arc<RwLock<HashMap<K, Arc<RwLock<V>>>>>,
}

impl<K, V> Cache<K, V>
    where K : std::cmp::Eq + std::hash::Hash {
    pub async fn get_entry_by_key<F>(&self, key: K, build_default_if_new: F) -> Arc<RwLock<V>>
        where F: Fn() -> V {
        let hs = self.hashes.read().await;
        let found = hs.get(&key);

        match found {
            Some(entry) => {
                return entry.clone();
            }

            None => {
                drop(hs);
                let mut write_hs = self.hashes.write().await;
                let default_if_new = Arc::new(RwLock::new(build_default_if_new()));
                (*write_hs).insert(key, default_if_new.clone());
                drop(write_hs);
                return default_if_new;
            }
        }
    }

    // It's Gracefully because if other thread still awaits for entry's write/read,
    // the entry neither the inside value are disposed, and the dispose only happens when the last thread disposes (Arc behavior).

    // Still, default_if_new for the same key after this call inserts a pointer to a different reference at the parent HashMap
    pub async fn destroy_gracefully_entry_by_key(&self, key: K) {
        let mut write_hs = self.hashes.write().await;
        let found = write_hs.get(&key);

        match found {
            Some(entry) => {
                let entry_clone = entry.clone();
                let mut _write = entry_clone.write().await;
                (*write_hs).remove(&key);
            }
            None => { }
        }
    }
}

pub fn new_empty_cache<K, V>() -> Cache<K, V>
    where K : std::cmp::Eq + std::hash::Hash {
    Cache {
        hashes: Arc::new(RwLock::new(HashMap::<K, Arc<RwLock<V>>>::new()))
    }
}
