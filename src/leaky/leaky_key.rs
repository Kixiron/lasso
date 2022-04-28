use crate::arenas::ConcurrentArena;
use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

pub trait LeakyKey:
    Sized + Copy + Eq + PartialEq<str> + AsRef<str> + Debug + sealed::Sealed
{
    type Arena: ConcurrentArena<Stored = Self::Ptr>;
    type Ptr;

    const SUPPORTS_INLINING: bool;

    fn empty() -> Self;

    fn is_empty(&self) -> bool;

    fn try_from_inline_str(string: &str) -> Option<Self>;

    fn from_ptr(ptr: Self::Ptr) -> Self;
}

mod sealed {
    use crate::leaky::{InlineStr, ThinStr};

    pub trait Sealed {}

    impl Sealed for InlineStr {}

    impl Sealed for ThinStr {}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct HashAsStr<T>(pub(super) T);

impl<T> PartialEq for HashAsStr<T>
where
    T: AsRef<str>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}

impl<T> Eq for HashAsStr<T> where T: AsRef<str> {}

impl<T> Hash for HashAsStr<T>
where
    T: AsRef<str>,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state)
    }
}

impl<T> Debug for HashAsStr<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
