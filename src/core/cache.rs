use std::sync::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use crate::core::index::UtfId;

pub struct Cache {
    inner: Mutex<LruCache<UtfId, Arc<str>>>,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn get(&self, id: UtfId) -> Option<Arc<str>> {
        self.inner.lock().unwrap().get(&id).cloned()
    }

    pub fn put(&self, id: UtfId, val: Arc<str>) {
        self.inner.lock().unwrap().put(id, val);
    }
}
