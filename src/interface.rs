#[cfg(feature = "multi-threaded")]
use crate::ThreadedRodeo;
use crate::{Key, Rodeo, RodeoReader, RodeoResolver};
use core::hash::BuildHasher;
#[cfg(feature = "multi-threaded")]
use core::hash::Hash;
use sealed::Sealed;

/// A generic interface that allows using any underlying interner for
/// both its reading and resolution capabilities, allowing both
/// `str -> key` and `key -> str` lookups
pub trait Reader<K>: Resolver<K> + Sealed {
    /// Get a key for the given string value if it exists
    fn get(&self, val: &str) -> Option<K>;

    /// Returns `true` if the current interner contains the given string
    fn contains(&self, val: &str) -> bool;
}

/// A generic interface that allows using any underlying interner only
/// for its resolution capabilities, allowing only `key -> str` lookups
pub trait Resolver<K>: Sealed {
    /// Resolves the given key into a string, panicking if it cannot be found
    fn resolve<'a>(&'a self, key: &K) -> &'a str;

    /// Attempts to resolve the given key into a string, returning `None`
    /// if it cannot be found
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str>;

    /// Returns `true` if the current interner contains the given key
    fn contains_key(&self, key: &K) -> bool;
}

impl<K, S> Reader<K> for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

impl<K, S> Resolver<K> for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<K, S> Reader<K> for RodeoReader<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

impl<K, S> Resolver<K> for RodeoReader<K, S>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<K> Resolver<K> for RodeoResolver<K>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

#[cfg(feature = "multi-threaded")]
impl<K, S> Reader<K> for ThreadedRodeo<K, S>
where
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

#[cfg(feature = "multi-threaded")]
impl<K, S> Resolver<K> for ThreadedRodeo<K, S>
where
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl<K, S> Sealed for Rodeo<K, S> {}
    impl<K> Sealed for RodeoResolver<K> {}
    impl<K, S> Sealed for RodeoReader<K, S> {}

    #[cfg(feature = "multi-threaded")]
    impl<K, S> Sealed for ThreadedRodeo<K, S> {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Spur;

    const INTERNED_STRINGS: &[&str] = &["foo", "bar", "baz", "biz", "buzz", "bing"];

    fn filled_rodeo() -> Rodeo {
        let mut rodeo = Rodeo::default();
        for string in INTERNED_STRINGS.iter().copied() {
            rodeo.try_get_or_intern_static(string).unwrap();
        }

        rodeo
    }

    #[cfg(feature = "multi-threaded")]
    fn filled_threaded_rodeo() -> ThreadedRodeo {
        let rodeo = ThreadedRodeo::default();
        for string in INTERNED_STRINGS.iter().copied() {
            rodeo.try_get_or_intern_static(string).unwrap();
        }

        rodeo
    }

    mod reader {
        use super::*;

        pub fn rodeo() -> Box<dyn Reader<Spur>> {
            Box::new(filled_rodeo())
        }

        pub fn rodeo_reader() -> Box<dyn Reader<Spur>> {
            Box::new(filled_rodeo().into_reader())
        }

        #[cfg(feature = "multi-threaded")]
        pub fn threaded_rodeo() -> Box<dyn Reader<Spur>> {
            Box::new(filled_threaded_rodeo())
        }
    }

    #[test]
    fn reader_implementations() {
        #[allow(unused_mut)]
        let mut readers = vec![reader::rodeo(), reader::rodeo_reader()];
        #[cfg(feature = "multi-threaded")]
        readers.push(reader::threaded_rodeo());

        for reader in readers {
            for (key, string) in INTERNED_STRINGS
                .iter()
                .copied()
                .enumerate()
                .map(|(i, s)| (Spur::try_from_usize(i).unwrap(), s))
            {
                assert!(reader.get(string).is_some());
                assert!(reader.contains(string));

                assert!(reader.contains_key(&key));
                assert_eq!(reader.resolve(&key), string);
                assert!(reader.try_resolve(&key).is_some());
                assert_eq!(reader.try_resolve(&key), Some(string));
            }
        }
    }

    mod resolver {
        use super::*;

        pub fn rodeo() -> Box<dyn Resolver<Spur>> {
            Box::new(filled_rodeo())
        }

        pub fn rodeo_reader() -> Box<dyn Resolver<Spur>> {
            Box::new(filled_rodeo().into_reader())
        }

        pub fn rodeo_resolver() -> Box<dyn Resolver<Spur>> {
            Box::new(filled_rodeo().into_resolver())
        }

        #[cfg(feature = "multi-threaded")]
        pub fn threaded_rodeo() -> Box<dyn Resolver<Spur>> {
            Box::new(filled_threaded_rodeo())
        }
    }

    #[test]
    fn resolver_implementations() {
        #[allow(unused_mut)]
        let mut resolvers = vec![
            resolver::rodeo(),
            resolver::rodeo_reader(),
            resolver::rodeo_resolver(),
        ];
        #[cfg(feature = "multi-threaded")]
        resolvers.push(resolver::threaded_rodeo());

        for resolver in resolvers {
            for (key, string) in INTERNED_STRINGS
                .iter()
                .copied()
                .enumerate()
                .map(|(i, s)| (Spur::try_from_usize(i).unwrap(), s))
            {
                assert!(resolver.contains_key(&key));
                assert_eq!(resolver.resolve(&key), string);
                assert!(resolver.try_resolve(&key).is_some());
                assert_eq!(resolver.try_resolve(&key), Some(string));
            }
        }
    }
}

// TODO: Figure out an interface that suits both `Rodeo`'s required mutability
//       and `ThreadedRodeo`'s interior mutability
//
// pub trait Interner<K>: Reader<K> + Resolver<K> {
//     fn get_or_intern(&mut self, val: &str) -> K;
//
//     fn try_get_or_intern(&mut self, val: &str) -> LassoResult<K>;
//
//     fn get_or_intern_static(&mut self, val: &'static str) -> K;
//
//     fn try_get_or_intern_static(&mut self, val: &'static str) -> LassoResult<K>;
// }
