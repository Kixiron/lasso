#![cfg(test)]

use super::*;
use crate::{Key, Spur};

compile! {
    if #[feature = "multi-threaded"] {
        use crate::ThreadedRodeo;
    }

    if #[feature = "no-std"] {
        use alloc::{boxed::Box, vec};
    }
}

const INTERNED_STRINGS: &[&str] = &["foo", "bar", "baz", "biz", "buzz", "bing"];
const UNINTERNED_STRINGS: &[&str] = &["rodeo", "default", "string", "static", "unwrap", "array"];

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

mod interner {
    use super::*;

    pub fn rodeo() -> Box<dyn Interner<Spur>> {
        Box::new(filled_rodeo())
    }

    #[cfg(feature = "multi-threaded")]
    pub fn threaded_rodeo() -> Box<dyn Interner<Spur>> {
        Box::new(filled_threaded_rodeo())
    }
}

#[test]
fn interner_implementations() {
    #[allow(unused_mut)]
    let mut interners = vec![interner::rodeo()];
    #[cfg(feature = "multi-threaded")]
    interners.push(interner::threaded_rodeo());

    for mut interner in interners {
        for (key, string) in INTERNED_STRINGS
            .iter()
            .copied()
            .enumerate()
            .map(|(i, s)| (Spur::try_from_usize(i).unwrap(), s))
        {
            assert!(interner.get(string).is_some());
            assert!(interner.contains(string));

            assert!(interner.contains_key(&key));
            assert_eq!(interner.resolve(&key), string);
            assert!(interner.try_resolve(&key).is_some());
            assert_eq!(interner.try_resolve(&key), Some(string));

            unsafe {
                assert_eq!(interner.resolve_unchecked(&key), string);
            }
        }

        for string in UNINTERNED_STRINGS.iter().copied() {
            let key = interner.get_or_intern(string);
            assert_eq!(interner.try_get_or_intern(string), Ok(key));
            assert_eq!(interner.get_or_intern_static(string), key);
            assert_eq!(interner.try_get_or_intern_static(string), Ok(key));

            assert!(interner.get(string).is_some());
            assert!(interner.contains(string));

            assert!(interner.contains_key(&key));
            assert_eq!(interner.resolve(&key), string);
            assert!(interner.try_resolve(&key).is_some());
            assert_eq!(interner.try_resolve(&key), Some(string));

            unsafe {
                assert_eq!(interner.resolve_unchecked(&key), string);
            }
        }

        let reader = interner.into_reader();
        for (key, string) in INTERNED_STRINGS
            .iter()
            .chain(UNINTERNED_STRINGS.iter())
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

            unsafe {
                assert_eq!(reader.resolve_unchecked(&key), string);
            }
        }
    }
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

            unsafe {
                assert_eq!(reader.resolve_unchecked(&key), string);
            }
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

            unsafe {
                assert_eq!(resolver.resolve_unchecked(&key), string);
            }
        }
    }
}
