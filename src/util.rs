use crate::{Key, Rodeo, RodeoReader, RodeoResolver};

use core::{hash::BuildHasher, iter, marker::PhantomData, slice};

#[derive(Debug)]
pub struct Iter<'a, K: Key> {
    iter: iter::Enumerate<slice::Iter<'a, &'a str>>,
    __key: PhantomData<K>,
}

impl<'a, K: Key> Iter<'a, K> {
    #[inline]
    pub(crate) fn from_rodeo<H: BuildHasher + Clone>(rodeo: &'a Rodeo<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter().enumerate(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H: BuildHasher + Clone>(rodeo: &'a RodeoReader<K, H>) -> Self {
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

impl<'a, K: Key> Iterator for Iter<'a, K> {
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
pub struct Strings<'a, K: Key> {
    iter: slice::Iter<'a, &'a str>,
    __key: PhantomData<K>,
}

impl<'a, K: Key> Strings<'a, K> {
    #[inline]
    pub(crate) fn from_rodeo<H: BuildHasher + Clone>(rodeo: &'a Rodeo<K, H>) -> Self {
        Self {
            iter: rodeo.strings.iter(),
            __key: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn from_reader<H: BuildHasher + Clone>(rodeo: &'a RodeoReader<K, H>) -> Self {
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

impl<'a, K: Key> Iterator for Strings<'a, K> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|string| *string)
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
                ($meta,)
                $(else if #[$else_if_meta] {
                    $($else_if_item)*
                })* $(else {
                    $($else_item)*
                })?
            }
        )+
    };

    (@recurse
        ($($prev_metas:tt)*)
        ($new_meta:meta)
        $($rem:tt)*
    )=>{
        compile!{
            @inner
            ($($prev_metas)* $new_meta,)
            $($rem)*
        }
    };
    (@inner
        $prev_metas:tt
        else if #[$meta:meta] {
            $($else_if_item:item)*
        }
        $($rem:tt)*

    )=>{
        $(
            #[cfg(all(not(any $prev_metas),$meta))]
            $else_if_item
        )*

        compile!{@recurse $prev_metas ($meta) $($rem)* }


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
