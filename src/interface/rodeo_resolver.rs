//! Implementations of [`Resolver`] for [`RodeoResolver`]

use crate::{rodeo::Internable, Key, Resolver, RodeoResolver};

impl<T: Internable, K> Resolver<T, K> for RodeoResolver<T, K>
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
