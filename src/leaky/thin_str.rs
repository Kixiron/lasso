use crate::{arenas::ThinStrArena, leaky::LeakyKey};
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
    hash::{Hash, Hasher},
    mem::size_of,
    ops::Deref,
    ptr::{addr_of, NonNull},
};

static EMPTY_THIN_STR_INNER: ThinStrInner = ThinStrInner {
    length: 0,
    _data: [],
};

const _: () = assert!(size_of::<ThinStr>() == size_of::<usize>());
const _: () = assert!(size_of::<ThinStr>() == size_of::<Option<ThinStr>>());

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ThinStr {
    ptr: NonNull<ThinStrInner>,
}

impl ThinStr {
    #[inline]
    pub fn empty() -> Self {
        Self {
            // Safety: The const reference isn't null
            ptr: unsafe { NonNull::new_unchecked(&EMPTY_THIN_STR_INNER as *const _ as *mut _) },
        }
    }

    // TODO: Close to being const
    #[inline]
    pub fn len(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).length }
    }

    // TODO: Close to being const
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ptr == unsafe { NonNull::new_unchecked(&EMPTY_THIN_STR_INNER as *const _ as *mut _) }
            || self.len() == 0
    }

    // TODO: Really close to being const
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe {
            let ptr = addr_of!((*self.ptr.as_ptr())._data).cast::<u8>();
            let bytes = core::slice::from_raw_parts(ptr, self.len());

            // TODO: Const utf8 check
            debug_assert!(core::str::from_utf8(bytes).is_ok());

            // Safety: The pointed-to string is valid utf8
            core::str::from_utf8_unchecked(bytes)
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ThinStrPtr(pub(crate) NonNull<ThinStrInner>);

impl LeakyKey for ThinStr {
    type Arena = ThinStrArena;
    type Ptr = ThinStrPtr;

    const SUPPORTS_INLINING: bool = false;

    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn try_from_inline_str(_: &str) -> Option<Self> {
        None
    }

    #[inline]
    fn from_ptr(ThinStrPtr(ptr): Self::Ptr) -> Self {
        Self { ptr }
    }
}

impl Default for ThinStr {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for ThinStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr || self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for ThinStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<ThinStr> for str {
    #[inline]
    fn eq(&self, other: &ThinStr) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<&str> for ThinStr {
    #[inline]
    fn eq(&self, &other: &&str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<ThinStr> for &str {
    #[inline]
    fn eq(&self, other: &ThinStr) -> bool {
        *self == other.as_str()
    }
}

impl Eq for ThinStr {}

impl PartialOrd for ThinStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for ThinStr {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for ThinStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl AsRef<str> for ThinStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for ThinStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Debug for ThinStr {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for ThinStr {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

#[repr(C)]
pub(crate) struct ThinStrInner {
    pub(crate) length: usize,
    pub(crate) _data: [u8; 0],
}
