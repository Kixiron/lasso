use crate::{key::Key, reader::RodeoReader, resolver::RodeoResolver, single_threaded::Rodeo};
use core::{iter, marker::PhantomData, num::NonZeroUsize, slice};

/// The amount of strings and bytes that an interner can hold before reallocating
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capacity {
    /// The number of strings that will be allocated
    pub(crate) strings: usize,
    /// The number of bytes that will be allocated
    pub(crate) bytes: NonZeroUsize,
}

impl Capacity {
    /// Create a new `Capacity` with the number of strings that the interner will hold
    /// and the number of bytes that the interner will hold
    #[inline]
    pub fn new(strings: usize, bytes: NonZeroUsize) -> Self {
        Self { strings, bytes }
    }

    /// Create a new `Capacity` with the number of strings that the interner will hold
    #[inline]
    pub fn for_strings(strings: usize) -> Self {
        Self {
            strings,
            ..Self::default()
        }
    }

    /// Create a new `Capacity` with the number of bytes that the interner will hold
    #[inline]
    pub fn for_bytes(bytes: NonZeroUsize) -> Self {
        Self {
            bytes,
            ..Self::default()
        }
    }

    /// Produces the smallest `Capacity` with enough room for zero strings and a single byte
    #[inline]
    pub fn minimal() -> Self {
        Self {
            strings: 0,
            // Safety: 1 is not 0
            bytes: unsafe { NonZeroUsize::new_unchecked(1) },
        }
    }

    /// Returns the number of strings this capacity will allocate
    #[inline]
    pub fn strings(&self) -> usize {
        self.strings
    }

    /// Returns the number of bytes this capacity will allocate
    #[inline]
    pub fn bytes(&self) -> NonZeroUsize {
        self.bytes
    }
}

/// Creates a `Capacity` that will hold 50 strings and 4096 bytes
impl Default for Capacity {
    #[inline]
    fn default() -> Self {
        Self {
            strings: 50,
            // Safety: 4096 is not 0
            bytes: unsafe { NonZeroUsize::new_unchecked(4096) },
        }
    }
}

/// Settings for the memory consumption of an interner
///
/// By default `max_memory_usage` is set to `usize::MAX`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryLimits {
    /// The maximum memory an interner will allocate
    pub(crate) max_memory_usage: usize,
}

impl MemoryLimits {
    /// Create a new `MemoryLimits` with the number of bytes that the interner can allocate
    #[inline]
    pub fn new(max_memory_usage: usize) -> Self {
        Self { max_memory_usage }
    }

    /// Create a new `MemoryLimits` with the number of bytes that the interner can allocate
    #[inline]
    pub fn for_memory_usage(max_memory_usage: usize) -> Self {
        Self {
            max_memory_usage,
            ..Self::default()
        }
    }

    /// Returns the maximum memory usage this `MemoryLimits` can allocate
    #[inline]
    pub fn max_memory_usage(&self) -> usize {
        self.max_memory_usage
    }
}

/// Creates a `MemoryLimits` with `max_memory_usage` set to `usize::max_value()`
impl Default for MemoryLimits {
    #[inline]
    fn default() -> Self {
        Self {
            max_memory_usage: usize::max_value(),
        }
    }
}

/// An iterator over an interner's strings and keys
#[derive(Debug)]
pub struct Iter<'a, K> {
    iter: iter::Enumerate<slice::Iter<'a, &'a str>>,
    __key: PhantomData<K>,
}

impl<'a, K> Iter<'a, K> {
    #[inline]
    pub(crate) fn from_rodeo<H>(rodeo: &'a Rodeo<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H>(rodeo: &'a RodeoReader<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_resolver(rodeo: &'a RodeoResolver<K>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }
}

impl<'a, K> Iterator for Iter<'a, K>
where
    K: Key,
{
    type Item = (K, &'a str);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, string)| {
            (
                K::try_from_usize(key).unwrap_or_else(|| unreachable!()),
                *string,
            )
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// #[derive(Debug)]
// pub struct LockedIter<'a, K: Key> {
//     iter: iter::Enumerate<slice::Iter<'a, &'a str>>,
//     #[cfg(not(feature = "parking_locks"))]
//     __guard: std::sync::MutexGuard<'a, Vec<&'static str>>,
//     __key: PhantomData<K>,
// }
//
// impl<'a, K: Key> LockedIter<'a, K> {
//     #[inline]
//     fn from_threaded<H: BuildHasher + Clone>(rodeo: &'a ThreadedRodeo<K, H>) -> Self {
//         let guard = rodeo.strings.lock().unwrap();
//
//         Self {
//             iter: guard.iter().enumerate(),
//             #[cfg(not(feature = "parking_locks"))]
//             __guard: guard,
//             __key: PhantomData,
//         }
//     }
// }

/// An iterator over an interner's strings
#[derive(Debug)]
pub struct Strings<'a, K> {
    iter: slice::Iter<'a, &'a str>,
    __key: PhantomData<K>,
}

impl<'a, K> Strings<'a, K> {
    #[inline]
    pub(crate) fn from_rodeo<H>(rodeo: &'a Rodeo<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H>(rodeo: &'a RodeoReader<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_resolver(rodeo: &'a RodeoResolver<K>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }
}

impl<'a, K> Iterator for Strings<'a, K> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|&k| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

macro_rules! compile {
    ($(
        if #[$meta:meta] {
            $($item:item)*
        } $(else if #[$else_if_meta:meta] {
            $($else_if_item:item)*
        })* $(else {
            $($else_item:item)*
        })?
    )+) => {
        $(
            $(
                #[cfg($meta)]
                $item
            )*

            compile!{
                @inner
                ( $meta, )
                $(else if #[$else_if_meta] {
                    $( $else_if_item )*
                })* $(else {
                    $( $else_item )*
                })?
            }
        )+
    };

    (@recurse
        ($($prev_metas:tt)*)
        ($new_meta:meta)
        $($rem:tt)*
    ) => {
        compile!{
            @inner
            ($( $prev_metas )* $new_meta,)
            $( $rem )*
        }
    };

    (@inner
        $prev_metas:tt
        else if #[$meta:meta] {
            $($else_if_item:item)*
        }
        $($rem:tt)*

    ) => {
        $(
            #[cfg(all(not(any $prev_metas), $meta))]
            $else_if_item
        )*

        compile! {
            @recurse $prev_metas ($meta) $( $rem )*
        }
    };

    (@inner
        $prev_metas:tt
        else {
            $($else_item:item)*
        }
    )=>{
        $(
            #[cfg(not(any $prev_metas))]
            $else_item
        )*
    };

    (@inner ($($prev_metas:tt)*))=>{};
}

#[cfg(debug_assertions)]
macro_rules! index_unchecked {
    ($slice:expr, $idx:expr) => {{
        let elem: &_ = $slice[$idx];
        elem
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! index_unchecked {
    ($slice:expr, $idx:expr) => {{
        let elem: &_ = $slice.get_unchecked($idx);
        elem
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_capacity() {
        let capacity = Capacity::new(100, NonZeroUsize::new(100).unwrap());
        assert_eq!(100, capacity.strings());
        assert_eq!(100, capacity.bytes.get());

        let capacity = Capacity::default();
        assert_eq!(capacity.strings, capacity.strings());
        assert_eq!(capacity.bytes, capacity.bytes());

        let capacity = Capacity::for_strings(10);
        assert_eq!(capacity.strings(), 10);

        let capacity = Capacity::for_bytes(NonZeroUsize::new(10).unwrap());
        assert_eq!(capacity.bytes().get(), 10);

        let capacity = Capacity::minimal();
        assert_eq!(capacity.strings(), 0);
        assert_eq!(capacity.bytes().get(), 1);
    }

    #[test]
    fn iter_rodeo() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern("A");
        let b = rodeo.get_or_intern("B");
        let c = rodeo.get_or_intern("C");
        let d = rodeo.get_or_intern("D");

        let mut iter = Iter::from_rodeo(&rodeo);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some((a, "A")), iter.next());
        assert_eq!(Some((b, "B")), iter.next());
        assert_eq!(Some((c, "C")), iter.next());
        assert_eq!(Some((d, "D")), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }

    #[test]
    fn iter_reader() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern("A");
        let b = rodeo.get_or_intern("B");
        let c = rodeo.get_or_intern("C");
        let d = rodeo.get_or_intern("D");

        let reader = rodeo.into_reader();
        let mut iter = Iter::from_reader(&reader);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some((a, "A")), iter.next());
        assert_eq!(Some((b, "B")), iter.next());
        assert_eq!(Some((c, "C")), iter.next());
        assert_eq!(Some((d, "D")), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }

    #[test]
    fn iter_resolver() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern("A");
        let b = rodeo.get_or_intern("B");
        let c = rodeo.get_or_intern("C");
        let d = rodeo.get_or_intern("D");

        let resolver = rodeo.into_resolver();
        let mut iter = Iter::from_resolver(&resolver);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some((a, "A")), iter.next());
        assert_eq!(Some((b, "B")), iter.next());
        assert_eq!(Some((c, "C")), iter.next());
        assert_eq!(Some((d, "D")), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }

    #[test]
    fn strings_rodeo() {
        let mut rodeo = Rodeo::default();
        rodeo.get_or_intern("A");
        rodeo.get_or_intern("B");
        rodeo.get_or_intern("C");
        rodeo.get_or_intern("D");

        let mut iter = Strings::from_rodeo(&rodeo);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some("A"), iter.next());
        assert_eq!(Some("B"), iter.next());
        assert_eq!(Some("C"), iter.next());
        assert_eq!(Some("D"), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }

    #[test]
    fn strings_reader() {
        let mut rodeo = Rodeo::default();
        rodeo.get_or_intern("A");
        rodeo.get_or_intern("B");
        rodeo.get_or_intern("C");
        rodeo.get_or_intern("D");

        let reader = rodeo.into_reader();
        let mut iter = Strings::from_reader(&reader);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some("A"), iter.next());
        assert_eq!(Some("B"), iter.next());
        assert_eq!(Some("C"), iter.next());
        assert_eq!(Some("D"), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }

    #[test]
    fn strings_resolver() {
        let mut rodeo = Rodeo::default();
        rodeo.get_or_intern("A");
        rodeo.get_or_intern("B");
        rodeo.get_or_intern("C");
        rodeo.get_or_intern("D");

        let resolver = rodeo.into_resolver();
        let mut iter = Strings::from_resolver(&resolver);

        assert_eq!((4, Some(4)), iter.size_hint());
        assert_eq!(Some("A"), iter.next());
        assert_eq!(Some("B"), iter.next());
        assert_eq!(Some("C"), iter.next());
        assert_eq!(Some("D"), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!((0, Some(0)), iter.size_hint());
    }
}
