use std::collections::BTreeMap;
use std::hash::Hasher;

use mea::rwlock::RwLock;

pub trait ShardDataKey: Ord + Clone {
    fn get_shard_idx(&self, totcap: usize) -> usize;
}

pub struct ShardedLockedData<K, T> {
    shards: Vec<RwLock<BTreeMap<K, T>>>,
}

impl<K: ShardDataKey, T> ShardedLockedData<K, T> {
    pub fn new(cap: usize) -> ShardedLockedData<K, T> {
        let mut shards = vec![];
        for _ in 0..cap {
            shards.push(RwLock::new(BTreeMap::new()));
        }
        ShardedLockedData { shards }
    }

    fn get_shard(&self, key: &K) -> &RwLock<BTreeMap<K, T>> {
        let idx = key.get_shard_idx(self.shards.len());
        debug_assert!(idx < self.shards.len());
        self.shards.get(idx).unwrap()
    }

    pub async fn contains_key(&self, key: &K) -> bool {
        let shard = self.get_shard(key);
        shard.read().await.contains_key(key)
    }

    pub async fn insert(&self, key: K, val: T) -> Option<T> {
        let shard = self.get_shard(&key);
        shard.write().await.insert(key, val)
    }

    pub async fn remove(&self, key: &K) -> Option<T> {
        let shard = self.get_shard(key);
        shard.write().await.remove(key)
    }

    pub async fn map<F, V>(&self, key: &K, f: F) -> Option<V>
    where
        F: FnOnce(&mut T) -> V,
    {
        let shard = self.get_shard(key);
        let mut sref = shard.write().await;
        let val = sref.get_mut(key)?;
        Some(f(val))
    }

    pub async fn get_all_keys(&self) -> Vec<K> {
        let mut result = vec![];
        for shard in self.shards.iter() {
            result.extend(shard.read().await.keys().cloned())
        }
        result.sort();
        result
    }
}

impl<K: ShardDataKey, T: Clone> ShardedLockedData<K, T> {
    pub async fn clone_val(&self, key: &K) -> Option<T> {
        let shard = self.get_shard(key);
        shard.write().await.get(key).cloned()
    }
}

impl ShardDataKey for u16 {
    fn get_shard_idx(&self, totcap: usize) -> usize {
        let sep = (u16::MAX as usize) / totcap;
        (*self as usize) / sep
    }
}

impl ShardDataKey for u64 {
    fn get_shard_idx(&self, totcap: usize) -> usize {
        let sep = (u64::MAX as usize) / totcap;
        (*self as usize) / sep
    }
}

impl ShardDataKey for crate::player::PlayerKey {
    fn get_shard_idx(&self, totcap: usize) -> usize {
        let mut h = std::hash::DefaultHasher::new();
        h.write(self);
        let n = h.finish() as usize;
        let sep = usize::MAX / totcap;
        n / sep
    }
}

impl ShardDataKey for String {
    fn get_shard_idx(&self, totcap: usize) -> usize {
        let mut h = std::hash::DefaultHasher::new();
        h.write(self.as_bytes());
        let n = h.finish() as usize;
        let sep = usize::MAX / totcap;
        let res = n / sep;
        log::debug!("shard data key {self} = {res} (totcap {totcap})");
        res
    }
}
