mod inline_str;
mod leaky_key;
mod thin_str;

pub use inline_str::InlineStr;
pub use leaky_key::LeakyKey;
pub use thin_str::ThinStr;

pub(crate) use thin_str::{ThinStrInner, ThinStrPtr};

use crate::{
    arenas::ConcurrentArena, hasher::RandomState, leaky::leaky_key::HashAsStr, LassoResult,
};
use core::{
    fmt::{self, Debug},
    hash::{BuildHasher, Hash, Hasher},
    mem::ManuallyDrop,
    num::NonZeroUsize,
};
use dashmap::{DashMap, SharedValue};
use hashbrown::hash_map::RawEntryMut;

pub struct LeakyRodeo<K = ThinStr, S = RandomState>
where
    K: LeakyKey,
{
    strings: DashMap<HashAsStr<K>, (), S>,
    arena: ManuallyDrop<K::Arena>,
}

impl<K, S> LeakyRodeo<K, S>
where
    K: LeakyKey,
    S: BuildHasher + Clone,
{
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            strings: DashMap::with_hasher(S::default()),
            arena: ManuallyDrop::new(
                <K::Arena>::new(NonZeroUsize::new(4096).unwrap(), usize::MAX).unwrap(),
            ),
        }
    }

    pub fn get_or_intern<T>(&self, string: T) -> LassoResult<K>
    where
        T: AsRef<str>,
    {
        let string = string.as_ref();

        // If the given string is empty, return an empty string
        if string.is_empty() {
            return Ok(K::empty());
        }

        // If the current key type supports inlining, attempt to create an inlined string
        if K::SUPPORTS_INLINING {
            if let Some(inlined) = K::try_from_inline_str(string) {
                return Ok(inlined);
            }
        }

        let hash = {
            let mut state = self.strings.hasher().build_hasher();
            string.hash(&mut state);
            state.finish()
        };
        let shard_idx = self.strings.determine_shard(hash as usize);

        let shards = self.strings.shards();
        debug_assert!(shard_idx < shards.len());
        let shard = unsafe { shards.get_unchecked(shard_idx) };

        let mut shard = shard.write();

        // FIXME: Raw entry api to reuse hash and not require allocating
        //        the string when the value already exists
        let entry = shard
            .raw_entry_mut()
            .from_hash(hash, |&HashAsStr(key)| key == *string);

        match entry {
            RawEntryMut::Occupied(occupied) => Ok(occupied.key().0),

            RawEntryMut::Vacant(vacant) => {
                debug_assert_ne!(string.len(), 0);
                // Safety: `string`'s length is greater than zero
                let key = K::from_ptr(unsafe { self.arena.store_str(string)? });

                // Insert the key into the shard
                vacant.insert_hashed_nocheck(hash, HashAsStr(key), SharedValue::new(()));

                Ok(key)
            }
        }
    }

    pub unsafe fn clear(self) {
        unsafe { self.arena.clear() };
    }
}

impl<K, S> Debug for LeakyRodeo<K, S>
where
    K: LeakyKey,
    S: BuildHasher + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[repr(transparent)]
        struct DebugStrings<'a, K, S>(&'a DashMap<HashAsStr<K>, (), S>);

        impl<'a, K, S> Debug for DebugStrings<'a, K, S>
        where
            K: LeakyKey,
            HashAsStr<K>: Hash,
            S: BuildHasher + Clone,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_set()
                    .entries(self.0.iter().map(|entry| entry.key().0))
                    .finish()
            }
        }

        f.debug_struct("LeakyRodeo")
            .field("strings", &DebugStrings(&self.strings))
            .field("arena", &self.arena)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::leaky::{InlineStr, LeakyRodeo};

    #[test]
    fn smoke_test_thin() {
        let interner: LeakyRodeo = LeakyRodeo::new();

        let empty = interner.get_or_intern("").unwrap();
        assert_eq!(empty, "");
        assert!(empty.is_empty());

        let whee1 = interner.get_or_intern("whee").unwrap();
        let whee2 = interner.get_or_intern("whee").unwrap();

        assert_eq!(whee1, "whee");
        assert_eq!(whee2, "whee");
        assert_eq!(whee1, whee2);

        let whoo = interner.get_or_intern("whoo").unwrap();
        assert_eq!(whoo, "whoo");

        unsafe { interner.clear() };
    }

    #[test]
    fn smoke_test_inline() {
        let interner: LeakyRodeo<InlineStr> = LeakyRodeo::new();

        let empty = interner.get_or_intern("").unwrap();
        assert_eq!(empty, "");
        assert!(empty.is_empty());

        let whee1 = interner.get_or_intern("whee").unwrap();
        let whee2 = interner.get_or_intern("whee").unwrap();

        assert_eq!(whee1, "whee");
        assert_eq!(whee2, "whee");
        assert_eq!(whee1, whee2);

        let whoo = interner.get_or_intern("whoo").unwrap();
        assert_eq!(whoo, "whoo");

        let long_whee = "wheeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let whee1 = interner.get_or_intern(long_whee).unwrap();
        let whee2 = interner.get_or_intern(long_whee).unwrap();

        assert_eq!(whee1, long_whee);
        assert_eq!(whee2, long_whee);
        assert_eq!(whee1, whee2);

        unsafe { interner.clear() };
    }
}
