use core::mem::{self, ManuallyDrop};
use dashmap::DashMap;
use std::{collections::HashMap, sync::RwLock};

pub trait Key: Eq {}

pub struct ReadOnlyLasso<K: Key> {
    map: HashMap<&'static str, K>,
    strings: Vec<&'static str>,
}

pub struct Lasso<K: Key> {
    map: DashMap<&'static str, K>,
    strings: RwLock<Vec<&'static str>>,
    __abort: AbortOnPanic,
}

impl<K: Key> Drop for Lasso<K> {
    fn drop(&mut self) {
        for map in self.map.shards() {
            map.write().drain().for_each(drop)
        }

        for string in self.strings.write().unwrap().drain(..) {
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

impl<K: Key> Lasso<K> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            strings: RwLock::new(Vec::new()),
            __abort: AbortOnPanic,
        }
    }

    pub fn into_read_only(self) -> ReadOnlyLasso<K> {
        let mut lasso = ManuallyDrop::new(self);

        let strings = mem::replace(&mut *lasso.strings.write().unwrap(), Vec::new());

        let mut map: HashMap<&'static str, K> = HashMap::with_capacity(strings.len());
        for shard in lasso.map.shards() {
            map.extend(shard.write().drain().map(|(k, v)| (k, v.into_inner())));
        }

        ReadOnlyLasso { map, strings }
    }
}

unsafe impl<K: Key> Send for Lasso<K> {}
unsafe impl<K: Key> Sync for Lasso<K> {}

struct AbortOnPanic;

impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() {
            std::process::abort();
        }
    }
}
