use super::{Interner, IntoReader, IntoResolver, Reader, Resolver};
use crate::{rodeo::Internable, Key, LassoResult};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;

impl<T: Internable, K, I> Interner<T, K> for Box<I>
where
    K: Key,
    I: Interner<T, K> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &T::Ref) -> K {
        (**self).get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &T::Ref) -> LassoResult<K> {
        (**self).try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static T::Ref) -> K {
        (**self).get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static T::Ref) -> LassoResult<K> {
        self.try_get_or_intern(val)
    }
}

impl<T: Internable, K, I> IntoReader<T, K> for Box<I>
where
    K: Key,
    I: IntoReader<T, K> + ?Sized + 'static,
{
    type Reader = <I as IntoReader<T, K>>::Reader;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_reader(self) -> Self::Reader
    where
        Self: 'static,
    {
        I::into_reader_boxed(self)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_reader_boxed(self: Box<Self>) -> Self::Reader
    where
        Self: 'static,
    {
        (*self).into_reader()
    }
}

impl<T: Internable, K, I> Reader<T, K> for Box<I>
where
    K: Key,
    I: Reader<T, K> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &T::Ref) -> Option<K> {
        (**self).get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &T::Ref) -> bool {
        (**self).contains(val)
    }
}

impl<T: Internable, K, I> IntoResolver<T, K> for Box<I>
where
    K: Key,
    I: IntoResolver<T, K> + ?Sized + 'static,
{
    type Resolver = <I as IntoResolver<T, K>>::Resolver;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver(self) -> Self::Resolver
    where
        Self: 'static,
    {
        I::into_resolver_boxed(self)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver_boxed(self: Box<Self>) -> Self::Resolver
    where
        Self: 'static,
    {
        (*self).into_resolver()
    }
}

impl<T: Internable, K, I> Resolver<T, K> for Box<I>
where
    K: Key,
    I: Resolver<T, K> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a T::Ref {
        (**self).resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a T::Ref> {
        (**self).try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a T::Ref {
        unsafe { (**self).resolve_unchecked(key) }
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        (**self).contains_key(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn len(&self) -> usize {
        (**self).len()
    }
}
