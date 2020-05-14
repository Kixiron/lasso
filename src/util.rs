use crate::{Key, Rodeo, RodeoReader, RodeoResolver};

use core::{hash::BuildHasher, iter, marker::PhantomData, slice};

#[derive(Debug)]
pub struct Iter<'a, 'unique, K: Key<'unique>> {
    iter: iter::Enumerate<slice::Iter<'a, &'a str>>,
    __key: PhantomData<&'unique K>,
}

impl<'a, 'unique, K: Key<'unique>> Iter<'a, 'unique, K> {
    #[inline]
    pub(crate) fn from_rodeo<H: BuildHasher + Clone>(rodeo: &'a Rodeo<'unique, K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H: BuildHasher + Clone>(
        rodeo: &'a RodeoReader<'unique, K, H>,
    ) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_resolver(rodeo: &'a RodeoResolver<'unique, K>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }
}

impl<'a, 'unique, K: Key<'unique>> Iterator for Iter<'a, 'unique, K> {
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

#[derive(Debug)]
pub struct Strings<'a, 'unique, K: Key<'unique>> {
    iter: slice::Iter<'a, &'a str>,
    __key: PhantomData<&'unique K>,
}

impl<'a, 'unique, K: Key<'unique>> Strings<'a, 'unique, K> {
    #[inline]
    pub(crate) fn from_rodeo<H: BuildHasher + Clone>(rodeo: &'a Rodeo<'unique, K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H: BuildHasher + Clone>(
        rodeo: &'a RodeoReader<'unique, K, H>,
    ) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_resolver(rodeo: &'a RodeoResolver<'unique, K>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }
}

impl<'a, 'unique, K: Key<'unique>> Iterator for Strings<'a, 'unique, K> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
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
macro_rules! compile_expr {
    ($(
        if #[$meta:meta] {
            $( $stmt:tt )*
        } $(else if #[$else_if_meta:meta] {
            $( $else_if_stmt:tt )*
        })* $(else {
            $( $else_stmt:tt )*
        })?
    )+) => {
        $(
            #[cfg($meta)]
            {
                $( $stmt )*
            }

            compile_expr! {
                @inner
                ( $meta, )
                $(else if #[$else_if_meta] {
                    $( $else_if_stmt )*
                })* $(else {
                    $( $else_stmt )*
                })?
            }
        )+
    };

    (@recurse ($( $prev_metas:tt)* ) ($new_meta:meta) $( $rem:tt )*) => {
        compile_expr! {
            @inner
            ($( $prev_metas )* $new_meta,)
            $( $rem )*
        }
    };

    (@inner
        $prev_metas:tt
        else if #[$meta:meta] {
            $( $else_if_stmt:tt )*
        }
        $( $rem:tt )*
    ) => {
        #[cfg(all(not(any $prev_metas), $meta))]
        {
            $( $else_if_stmt )*
        }

        compile_expr! {
            @recurse $prev_metas ($meta) $( $rem )*
        }
    };

    (@inner $prev_metas:tt
        else {
            $( $else_stmt:tt )*
        }
    ) => {
        #[cfg(not(any $prev_metas))]
        {
            $( $else_stmt )*
        }
    };

    (@inner ($($prev_metas:tt)*)) => {};
}

#[cfg(test)]
mod tests {
    use super::*;

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
