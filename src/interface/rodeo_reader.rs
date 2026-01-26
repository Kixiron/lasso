//! Implementations of [`Reader`] and [`Resolver`] for [`RodeoReader`]

use crate::{rodeo::Internable, IntoResolver, Key, Reader, Resolver, RodeoReader, RodeoResolver};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::BuildHasher;

impl<T: Internable, K, S> Reader<T, K> for RodeoReader<T, K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &T::Ref) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &T::Ref) -> bool {
        self.contains(val)
    }
}

impl<T: Internable, K, S> IntoResolver<T, K> for RodeoReader<T, K, S>
where
    K: Key,
    S: BuildHasher,
{
    type Resolver = RodeoResolver<T, K>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver(self) -> Self::Resolver
    where
        Self: 'static,
    {
        self.into_resolver()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver_boxed(self: Box<Self>) -> Self::Resolver
    where
        Self: 'static,
    {
        RodeoReader::into_resolver(*self)
    }
}

impl<T: Internable, K, S> Resolver<T, K> for RodeoReader<T, K, S>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a T::Ref {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a T::Ref> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a T::Ref {
        unsafe { self.resolve_unchecked(key) }
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn len(&self) -> usize {
        self.len()
    }
}
