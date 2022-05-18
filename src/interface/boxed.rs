use super::{Interner, IntoReader, IntoResolver, Reader, Resolver};
use crate::{Internable, Key, LassoResult};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;

impl<K, V, I> Interner<K, V> for Box<I>
where
    K: Key,
    V: ?Sized + Internable,
    I: Interner<K, V> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &V) -> K {
        (&mut **self).get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &V) -> LassoResult<K> {
        (&mut **self).try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static V) -> K {
        (&mut **self).get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static V) -> LassoResult<K> {
        self.try_get_or_intern(val)
    }
}

impl<K, V, I> IntoReader<K, V> for Box<I>
where
    K: Key,
    V: ?Sized + Internable,
    I: IntoReader<K, V> + ?Sized + 'static,
{
    type Reader = <I as IntoReader<K, V>>::Reader;

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader(self) -> Self::Reader
    where
        Self: 'static,
    {
        I::into_reader_boxed(self)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader_boxed(self: Box<Self>) -> Self::Reader
    where
        Self: 'static,
    {
        (*self).into_reader()
    }
}

impl<K, V, I> Reader<K, V> for Box<I>
where
    K: Key,
    V: ?Sized + Internable,
    I: Reader<K, V> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &V) -> Option<K> {
        (&**self).get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &V) -> bool {
        (&**self).contains(val)
    }
}

impl<K, V, I> IntoResolver<K, V> for Box<I>
where
    K: Key,
    V: ?Sized + Internable,
    I: IntoResolver<K, V> + ?Sized + 'static,
{
    type Resolver = <I as IntoResolver<K, V>>::Resolver;

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver(self) -> Self::Resolver
    where
        Self: 'static,
    {
        I::into_resolver_boxed(self)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver_boxed(self: Box<Self>) -> Self::Resolver
    where
        Self: 'static,
    {
        (*self).into_resolver()
    }
}

impl<K, V, I> Resolver<K, V> for Box<I>
where
    K: Key,
    V: ?Sized + Internable,
    I: Resolver<K, V> + ?Sized + 'static,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a V {
        (&**self).resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a V> {
        (&**self).try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a V {
        unsafe { (&**self).resolve_unchecked(key) }
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        (&**self).contains_key(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn len(&self) -> usize {
        (&**self).len()
    }
}
