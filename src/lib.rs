use core::{
    mem::{self, ManuallyDrop},
    num::NonZeroUsize,
};
use dashmap::DashMap;
use std::{collections::HashMap, sync::RwLock};

pub struct Lasso<K: Key> {
    map: DashMap<&'static str, K>,
    strings: RwLock<Vec<&'static str>>,
    __abort: AbortOnPanic,
}

impl<K: Key> Lasso<K> {
    #[inline]
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            strings: RwLock::new(Vec::new()),
            __abort: AbortOnPanic,
        }
    }

    #[inline]
    pub fn intern<T>(&self, val: T) -> K
    where
        T: Into<String>,
    {
        let string = Box::leak(val.into().into_boxed_str());

        let key = {
            let mut strings = self.strings.write().unwrap();
            let key = K::from_usize(strings.len());
            strings.push(string);

            key
        };

        self.map.insert(string, key);

        key
    }

    #[inline]
    pub fn get_or_intern<T>(&self, val: T) -> K
    where
        T: Into<String> + AsRef<str>,
    {
        if let Some(key) = self.get(val.as_ref()) {
            key
        } else {
            self.intern(val.into())
        }
    }

    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).map(|k| *k)
    }

    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.strings
            .read()
            .unwrap()
            .get(key.into_usize())
            .map(|s| *s)
    }

    #[inline]
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str {
        self.strings.read().unwrap().get_unchecked(key.into_usize())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.strings.read().unwrap().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
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

unsafe impl<K: Key> Send for Lasso<K> {}
unsafe impl<K: Key> Sync for Lasso<K> {}

pub struct ReadOnlyLasso<K: Key> {
    map: HashMap<&'static str, K>,
    strings: Vec<&'static str>,
}

impl<K: Key> ReadOnlyLasso<K> {
    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).map(|k| *k)
    }

    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.strings.get(key.into_usize()).map(|s| *s)
    }

    #[inline]
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str {
        self.strings.get_unchecked(key.into_usize())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<K: Key> Drop for ReadOnlyLasso<K> {
    fn drop(&mut self) {
        self.map.drain().for_each(drop);

        for string in self.strings.drain(..) {
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

unsafe impl<K: Key> Send for ReadOnlyLasso<K> {}
unsafe impl<K: Key> Sync for ReadOnlyLasso<K> {}

pub trait Key: Copy + Eq {
    fn into_usize(self) -> usize;
    fn from_usize(int: usize) -> Self;
}

impl<T> Key for T
where
    T: Copy + Eq + From<usize> + Into<usize>,
{
    #[inline]
    fn into_usize(self) -> usize {
        self.into()
    }

    #[inline]
    fn from_usize(int: usize) -> Self {
        int.into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cord(NonZeroUsize);

impl Key for Cord {
    #[inline]
    fn into_usize(self) -> usize {
        self.0.get() + 1
    }

    #[inline]
    fn from_usize(int: usize) -> Self {
        Self(
            NonZeroUsize::new(int - 1)
                .expect("Can only use values up to `usize::MAX - 1` for Cord"),
        )
    }
}

struct AbortOnPanic;

impl Drop for AbortOnPanic {
    #[inline]
    fn drop(&mut self) {
        if std::thread::panicking() {
            std::process::abort();
        }
    }
}
