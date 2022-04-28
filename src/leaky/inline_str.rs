use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
    hash::{Hash, Hasher},
    mem::size_of,
    ops::Deref,
    ptr::NonNull,
};

use crate::{arenas::InlineStrArena, leaky::LeakyKey};

/// The total type size of [`InlineSpur`], 24 bytes
const ISTR_SIZE: usize = 24;

/// The maximum capacity of an inlined key's string
const INLINE_CAPACITY: usize = ISTR_SIZE - 2;

// Some static assertions to make sure everything's correct
const _: () = assert!(INLINE_CAPACITY <= u8::MAX as usize);
// Note: I've tried doing the song and dance to open this type
//       up to niche optimization but I don't think rustc propagates
//       niche info through unions
const _: () = assert!(size_of::<InlineStr>() == ISTR_SIZE);
const _: () = assert!(size_of::<IStrInner>() == ISTR_SIZE);
const _: () = assert!(size_of::<IStrKind>() == 1);
const _: () = assert!(size_of::<OutlineStrData>() == ISTR_SIZE);
const _: () = assert!(size_of::<OutlineStrData>() == ISTR_SIZE);

/// A key type that supports inlining small strings
///
/// Small strings (strings with a byte length of <= 22) are inlined into the
/// key instead of always being interned. Strings with a length of greater than
/// 22 are interned like normal and have their key stored
// TODO: `InlineStr` is zeroable, all zeroes correspond to an empty inline string.
//       Maybe add bytemuck behind a feature gate with `Zeroable` and `NoUninit` impls
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InlineStr {
    inner: IStrInner,
}

impl InlineStr {
    /// Create an empty [`InlineStr`] that refers to an inlined empty string
    #[inline]
    pub const fn empty() -> Self {
        Self {
            inner: IStrInner::inlined(0, [0; INLINE_CAPACITY]),
        }
    }

    /// Returns the string that the current [`InlineStr`] contains or points to
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl LeakyKey for InlineStr {
    type Arena = InlineStrArena;
    type Ptr = &'static str;

    const SUPPORTS_INLINING: bool = true;

    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    #[inline]
    fn try_from_inline_str(string: &str) -> Option<Self> {
        if string.len() <= INLINE_CAPACITY {
            let mut buf = [0; INLINE_CAPACITY];
            buf[..string.len()].copy_from_slice(string.as_bytes());

            Some(Self {
                inner: IStrInner::inlined(string.len() as u8, buf),
            })
        } else {
            None
        }
    }

    #[inline]
    fn from_ptr(string: Self::Ptr) -> Self {
        // Safety: References cannot be null
        debug_assert!(!string.as_ptr().is_null());
        let ptr = unsafe { NonNull::new_unchecked(string.as_ptr() as *mut u8) };

        Self {
            inner: IStrInner::outlined(ptr, string.len()),
        }
    }
}

impl PartialEq<str> for InlineStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<InlineStr> for str {
    #[inline]
    fn eq(&self, other: &InlineStr) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<&str> for InlineStr {
    #[inline]
    fn eq(&self, &other: &&str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<InlineStr> for &str {
    #[inline]
    fn eq(&self, other: &InlineStr) -> bool {
        *self == other.as_str()
    }
}

impl Default for InlineStr {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl AsRef<str> for InlineStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for InlineStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Debug for InlineStr {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for InlineStr {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum IStrKind {
    Inline = 0,
    Outline = 1,
}

impl IStrKind {
    #[inline]
    const fn is_outline(&self) -> bool {
        matches!(self, Self::Outline)
    }

    #[inline]
    const fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct OutlineStrData {
    kind: IStrKind,
    padding: [u8; Self::PADDING_LEN],
    ptr: NonNull<u8>,
    length: usize,
}

impl OutlineStrData {
    /// The length of the padding within `self.padding`
    const PADDING_LEN: usize = ISTR_SIZE
            // Discriminant
            - size_of::<IStrKind>()
            // Pointer
            - size_of::<NonNull<u8>>()
            // Length
            - size_of::<usize>();

    #[inline]
    const fn new(ptr: NonNull<u8>, length: usize) -> Self {
        debug_assert!(length > INLINE_CAPACITY);

        Self {
            kind: IStrKind::Outline,
            padding: [0; Self::PADDING_LEN],
            ptr,
            length,
        }
    }

    // TODO: This is *so* close to being const
    #[inline]
    fn as_str(&self) -> &str {
        // Safety: The pointed-to string is valid for `length` bytes
        // TODO: `slice::from_raw_parts()` isn't const yet
        let slice = unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.length) };

        // TODO: Const utf8 check
        debug_assert!(core::str::from_utf8(slice).is_ok());

        // Safety: The pointed-to string is valid utf8
        // TODO: debug assertion for utf8 validity
        unsafe { core::str::from_utf8_unchecked(slice) }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct InlineStrData {
    kind: IStrKind,
    length: u8,
    buf: [u8; INLINE_CAPACITY],
}

impl InlineStrData {
    #[inline]
    const fn new(length: u8, buf: [u8; INLINE_CAPACITY]) -> Self {
        debug_assert!(length as usize <= INLINE_CAPACITY);

        Self {
            kind: IStrKind::Inline,
            length,
            buf,
        }
    }

    // FIXME: This is *so* close to being const
    #[inline]
    fn as_str(&self) -> &str {
        // Safety: `ptr` (and therefore `self.buf`) is valid for `self.length` bytes
        // TODO: `slice::from_raw_parts()` is for some reason not const
        let slice =
            unsafe { core::slice::from_raw_parts(&self.buf as *const u8, self.length as usize) };

        // TODO: Const utf8 check
        debug_assert!(core::str::from_utf8(slice).is_ok());

        // Safety: The bytes up to `length` are valid utf8
        unsafe { core::str::from_utf8_unchecked(slice) }
    }
}

/// The underlying implementation of [`InlineSpur`]
#[derive(Clone, Copy)]
#[repr(C)]
union IStrInner {
    /// View only the spur's discriminant
    kind: IStrKind,

    /// An un-inlined key
    outline: OutlineStrData,

    /// An inlined string
    inline: InlineStrData,

    /// View the inner repr as a bag of bytes, cannot cast outlined
    /// variants into this
    bytes: [u8; ISTR_SIZE],
}

impl IStrInner {
    #[inline]
    const fn outlined(ptr: NonNull<u8>, length: usize) -> Self {
        Self {
            outline: OutlineStrData::new(ptr, length),
        }
    }

    #[inline]
    const fn inlined(length: u8, buf: [u8; INLINE_CAPACITY]) -> Self {
        Self {
            inline: InlineStrData::new(length, buf),
        }
    }

    #[inline]
    const fn discriminant(&self) -> IStrKind {
        // Safety: There's always a valid discriminant
        unsafe { self.kind }
    }

    #[inline]
    fn as_bytes(&self) -> [u8; ISTR_SIZE] {
        match self.discriminant() {
            // Safety: It's always valid to interpret the spur as bytes
            IStrKind::Inline => unsafe { self.bytes },

            // FIXME: The codegen here makes me very sad
            IStrKind::Outline => {
                let outline = unsafe { self.as_outline_unchecked() };
                let mut bytes = [0; ISTR_SIZE];

                bytes[0] = IStrKind::Outline as u8;
                bytes[1..1 + OutlineStrData::PADDING_LEN].copy_from_slice(&outline.padding);
                bytes[1 + OutlineStrData::PADDING_LEN
                    ..1 + OutlineStrData::PADDING_LEN + size_of::<usize>()]
                    .copy_from_slice(&(outline.ptr.as_ptr() as usize).to_ne_bytes());
                bytes[1 + OutlineStrData::PADDING_LEN + size_of::<usize>()
                    ..1 + OutlineStrData::PADDING_LEN + size_of::<usize>() + size_of::<usize>()]
                    .copy_from_slice(&outline.length.to_ne_bytes());

                bytes
            }
        }
    }

    #[inline]
    fn as_str(&self) -> &str {
        unsafe {
            match self.discriminant() {
                // Safety: We've checked that this is an outlined string
                IStrKind::Outline => self.as_outline_unchecked().as_str(),

                // Safety: We've checked that this is an inlined string
                IStrKind::Inline => self.as_inline_unchecked().as_str(),
            }
        }
    }

    #[inline]
    const unsafe fn as_inline_unchecked(&self) -> &InlineStrData {
        debug_assert!(self.discriminant().is_inline());
        unsafe { &self.inline }
    }

    #[inline]
    const unsafe fn as_outline_unchecked(&self) -> &OutlineStrData {
        debug_assert!(self.discriminant().is_outline());
        unsafe { &self.outline }
    }
}

impl PartialEq for IStrInner {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Compare the raw bytes of each spur
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for IStrInner {}

impl PartialOrd for IStrInner {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_bytes().partial_cmp(&other.as_bytes())
    }
}

impl Ord for IStrInner {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(&other.as_bytes())
    }
}

impl Hash for IStrInner {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the bytes of the spur
        self.as_bytes().hash(state);
    }
}
