use core::{
    fmt::{self, Debug, Write},
    num::{NonZeroU16, NonZeroU32, NonZeroU8, NonZeroUsize},
};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    mem::size_of,
};

/// Types implementing this trait can be used as keys for all Rodeos
///
/// # Safety
///
/// into/from must be perfectly symmetrical, any key that goes on must be perfectly reproduced with the other
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
pub unsafe trait Key: Copy + Eq {
    /// If `true`, the key type can support inline strings
    const SUPPORTS_INLINING: bool = false;

    /// Returns the `usize` that represents the current key
    fn into_usize(self) -> usize;

    // fn try_into_inlined(&self) -> Option<&str>;

    /// Attempts to create a key from a `usize`, returning `None` if it fails
    fn try_from_usize(key: usize) -> Option<Self>;

    /// Attempts to create a key from either a key or an inlined string
    #[inline]
    fn try_from_inlined(_string: &str) -> Option<Self> {
        None
    }

    /// Returns `true` if the current key is an inlined string
    #[inline]
    fn is_inline(&self) -> bool {
        false
    }
}

/// The total type size of [`InlineSpur`], 24 bytes
const INLINE_SPUR_SIZE: usize = 24;

/// The maximum capacity of an inlined key's string
const INLINE_CAPACITY: usize = INLINE_SPUR_SIZE - 2;

// Ensure the sizes of everything are correct
// Note: I've tried doing the song and dance to open this type
//       up to niche optimization but I don't think rustc propagates
//       niche info through unions
const _: () = assert!(size_of::<InlineSpur>() == INLINE_SPUR_SIZE);
const _: () = assert!(size_of::<InlineSpurInner>() == INLINE_SPUR_SIZE);
const _: () = assert!(size_of::<InlineSpurDiscriminant>() == 1);
const _: () = assert!(size_of::<InlineSpurKey>() == INLINE_SPUR_SIZE);
const _: () = assert!(size_of::<InlineSpurStr>() == INLINE_SPUR_SIZE);

/// A key type that supports inlining small strings
///
/// Small strings (strings with a byte length of <= 22) are inlined into the
/// key instead of always being interned. Strings with a length of greater than
/// 22 are interned like normal and have their key stored
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InlineSpur {
    inner: InlineSpurInner,
}

impl InlineSpur {
    /// Returns `true` if the current spur is an interned key
    #[inline]
    pub const fn is_interned_key(&self) -> bool {
        self.inner.discriminant().is_key()
    }

    /// Returns `true` if the current spur is an inlined string
    #[inline]
    pub const fn is_inlined_str(&self) -> bool {
        self.inner.discriminant().is_inline()
    }

    #[inline]
    const fn as_interned_key(&self) -> Option<LargeSpur> {
        if let Some(interned) = self.inner.as_key() {
            Some(interned.key)
        } else {
            None
        }
    }

    /// Returns the contained string if the current spur is an inline string
    // FIXME: This is *so* close to being const
    #[inline]
    pub fn as_inlined_str(&self) -> Option<&str> {
        if let Some(inlined) = self.inner.as_inline() {
            Some(inlined.as_str())
        } else {
            None
        }
    }

    /// Create an empty [`InlineSpur`] that refers to an inlined empty string
    #[inline]
    const fn empty() -> Self {
        Self {
            inner: InlineSpurInner::inlined(0, [0; INLINE_CAPACITY]),
        }
    }
}

unsafe impl Key for InlineSpur {
    const SUPPORTS_INLINING: bool = true;

    #[inline]
    fn into_usize(self) -> usize {
        self.as_interned_key()
            .expect("called `Key::into_usize()` on an inlined string")
            .into_usize()
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(key: usize) -> Option<Self> {
        LargeSpur::try_from_usize(key).map(|key| Self {
            inner: InlineSpurInner::key(key),
        })
    }

    #[inline]
    fn try_from_inlined(string: &str) -> Option<Self> {
        if string.len() <= INLINE_CAPACITY {
            debug_assert!(u8::try_from(string.len()).is_ok());

            // Copy the given string into the key's buffer
            let mut buf = [0; INLINE_CAPACITY];
            buf[..string.len()].copy_from_slice(string.as_bytes());

            // Create an inlined key from the given string
            Some(Self {
                inner: InlineSpurInner::inlined(string.len() as u8, buf),
            })
        } else {
            None
        }
    }

    #[inline]
    fn is_inline(&self) -> bool {
        self.is_inlined_str()
    }
}

impl Default for InlineSpur {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl Debug for InlineSpur {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InlineSpur(")?;
        match self.inner.discriminant() {
            InlineSpurDiscriminant::Key => {
                // Safety: We've checked that this is a key
                Debug::fmt(unsafe { &self.inner.key.key }, f)?;
            }

            InlineSpurDiscriminant::Inline => {
                // Safety: We've checked that this is an inlined string
                let inlined = unsafe { &self.inner.inline };
                Debug::fmt(inlined.as_str(), f)?;
            }
        }
        f.write_char(')')
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum InlineSpurDiscriminant {
    Key,
    Inline,
}

impl InlineSpurDiscriminant {
    #[inline]
    const fn is_key(&self) -> bool {
        matches!(self, Self::Key)
    }

    #[inline]
    const fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct InlineSpurKey {
    discriminant: InlineSpurDiscriminant,
    padding: [u8; Self::PADDING_LEN],
    key: LargeSpur,
}

impl InlineSpurKey {
    /// The length of the padding within `self.padding`
    const PADDING_LEN: usize =
        INLINE_SPUR_SIZE - size_of::<LargeSpur>() - size_of::<InlineSpurDiscriminant>();

    #[inline]
    const fn new(key: LargeSpur) -> Self {
        Self {
            discriminant: InlineSpurDiscriminant::Key,
            padding: [0; Self::PADDING_LEN],
            key,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct InlineSpurStr {
    discriminant: InlineSpurDiscriminant,
    length: u8,
    buf: [u8; INLINE_CAPACITY],
}

impl InlineSpurStr {
    #[inline]
    const fn new(length: u8, buf: [u8; INLINE_CAPACITY]) -> Self {
        Self {
            discriminant: InlineSpurDiscriminant::Inline,
            length,
            buf,
        }
    }

    // FIXME: This is *so* close to being const
    #[inline]
    fn as_str(&self) -> &str {
        let bytes = &self.buf[..self.length as usize];
        debug_assert!(core::str::from_utf8(bytes).is_ok());

        // Safety: The bytes up to `length` are valid utf8
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
}

/// The underlying implementation of [`InlineSpur`]
#[repr(C)]
union InlineSpurInner {
    /// View only the spur's discriminant
    discriminant: InlineSpurDiscriminant,

    /// An un-inlined key
    key: InlineSpurKey,

    /// An inlined string
    inline: InlineSpurStr,

    /// View the inner repr as a bag of bytes, all bytes are initialized
    bytes: [u8; INLINE_SPUR_SIZE],
}

impl InlineSpurInner {
    #[inline]
    const fn key(key: LargeSpur) -> Self {
        Self {
            key: InlineSpurKey::new(key),
        }
    }

    #[inline]
    const fn inlined(length: u8, buf: [u8; INLINE_CAPACITY]) -> Self {
        Self {
            inline: InlineSpurStr::new(length, buf),
        }
    }

    #[inline]
    const fn discriminant(&self) -> InlineSpurDiscriminant {
        // Safety: There's always a valid discriminant
        unsafe { self.discriminant }
    }

    #[inline]
    const fn as_bytes(&self) -> &[u8; INLINE_SPUR_SIZE] {
        // Safety: It's always valid to interpret the spur as bytes
        unsafe { &self.bytes }
    }

    #[inline]
    const fn as_inline(&self) -> Option<&InlineSpurStr> {
        if self.discriminant().is_inline() {
            // Safety: We've checked that the discriminant is `Inline`
            Some(unsafe { &self.inline })
        } else {
            None
        }
    }

    #[inline]
    const fn as_key(&self) -> Option<&InlineSpurKey> {
        if self.discriminant().is_key() {
            // Safety: We've checked that the discriminant is `Key`
            Some(unsafe { &self.key })
        } else {
            None
        }
    }
}

impl Clone for InlineSpurInner {
    #[inline]
    fn clone(&self) -> Self {
        // Directly copy the bytes of the spur into the new one
        Self {
            bytes: *self.as_bytes(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.bytes = *source.as_bytes();
    }
}

impl Copy for InlineSpurInner {}

impl PartialEq for InlineSpurInner {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Compare the raw bytes of each spur
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for InlineSpurInner {}

impl PartialOrd for InlineSpurInner {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl Ord for InlineSpurInner {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl Hash for InlineSpurInner {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the bytes of the spur
        self.as_bytes().hash(state);
    }
}

/// A key type taking up `size_of::<usize>()` bytes of space (generally 4 or 8 bytes)
///
/// Internally is a `NonZeroUsize` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LargeSpur {
    key: NonZeroUsize,
}

impl LargeSpur {
    /// Returns the [`NonZeroUsize`] backing the current `LargeSpur`
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn into_inner(self) -> NonZeroUsize {
        self.key
    }
}

unsafe impl Key for LargeSpur {
    #[inline]
    fn into_usize(self) -> usize {
        self.key.get() - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(key: usize) -> Option<Self> {
        if key < usize::max_value() {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroUsize
            unsafe {
                Some(Self {
                    key: NonZeroUsize::new_unchecked(key + 1),
                })
            }
        } else {
            None
        }
    }
}

impl Default for LargeSpur {
    #[inline]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl Debug for LargeSpur {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LargeSpur(")?;
        Debug::fmt(&self.key, f)?;
        f.write_char(')')
    }
}

/// The default key for every Rodeo, uses only 32 bits of space
///
/// Internally is a `NonZeroU32` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Spur {
    key: NonZeroU32,
}

impl Spur {
    /// Returns the [`NonZeroU32`] backing the current `Spur`
    #[inline]
    pub const fn into_inner(self) -> NonZeroU32 {
        self.key
    }
}

unsafe impl Key for Spur {
    #[inline]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u32::MAX - 1`
    #[inline]
    fn try_from_usize(key: usize) -> Option<Self> {
        if key < u32::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU32
            unsafe {
                Some(Self {
                    key: NonZeroU32::new_unchecked(key as u32 + 1),
                })
            }
        } else {
            None
        }
    }
}

impl Default for Spur {
    #[inline]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl Debug for Spur {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Spur(")?;
        Debug::fmt(&self.key, f)?;
        f.write_char(')')
    }
}

/// A miniature Key utilizing only 16 bits of space
///
/// Internally is a `NonZeroU16` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MiniSpur {
    key: NonZeroU16,
}

impl MiniSpur {
    /// Returns the [`NonZeroU16`] backing the current `MiniSpur`
    #[inline]
    pub const fn into_inner(self) -> NonZeroU16 {
        self.key
    }
}

unsafe impl Key for MiniSpur {
    #[inline]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u16::MAX - 1`
    #[inline]
    fn try_from_usize(key: usize) -> Option<Self> {
        if key < u16::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU16
            unsafe {
                Some(Self {
                    key: NonZeroU16::new_unchecked(key as u16 + 1),
                })
            }
        } else {
            None
        }
    }
}

impl Default for MiniSpur {
    #[inline]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl Debug for MiniSpur {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MiniSpur(")?;
        Debug::fmt(&self.key, f)?;
        f.write_char(')')
    }
}

/// A miniature Key utilizing only 8 bits of space
///
/// Internally is a `NonZeroU8` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MicroSpur {
    key: NonZeroU8,
}

impl MicroSpur {
    /// Returns the [`NonZeroU8`] backing the current `MicroSpur`
    #[inline]
    pub const fn into_inner(self) -> NonZeroU8 {
        self.key
    }
}

unsafe impl Key for MicroSpur {
    #[inline]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u8::MAX - 1`
    #[cfg_attr(feature = "inline-more", inline)]
    fn try_from_usize(key: usize) -> Option<Self> {
        if key < u8::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU8
            unsafe {
                Some(Self {
                    key: NonZeroU8::new_unchecked(key as u8 + 1),
                })
            }
        } else {
            None
        }
    }
}

impl Default for MicroSpur {
    #[inline]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl Debug for MicroSpur {
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MicroSpur(")?;
        Debug::fmt(&self.key, f)?;
        f.write_char(')')
    }
}

macro_rules! impl_serde {
    ($($key:ident => $ty:ident),* $(,)?) => {
        #[cfg(feature = "serialize")]
        mod __serde {
            use super::{$($key),*};
            use serde::{
                de::{Deserialize, Deserializer},
                ser::{Serialize, Serializer},
            };
            use core::num::{$($ty),*};

            $(
                impl Serialize for $key {
                    #[cfg_attr(feature = "inline-more", inline)]
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        self.key.serialize(serializer)
                    }
                }

                impl<'de> Deserialize<'de> for $key {
                    #[cfg_attr(feature = "inline-more", inline)]
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let key = <$ty>::deserialize(deserializer)?;
                        Ok(Self { key })
                    }
                }
            )*
        }
    };
}

// Implement `Serialize` and `Deserialize` when the `serde` feature is enabled
impl_serde! {
    Spur => NonZeroU32,
    MiniSpur => NonZeroU16,
    MicroSpur => NonZeroU8,
    LargeSpur => NonZeroUsize,
}

macro_rules! impl_deepsize {
    ($($type:ident),* $(,)?) => {
        #[cfg(feature = "deepsize")]
        mod __deepsize {
            use super::{$($type),*};
            #[cfg(test)]
            use super::Key;
            use deepsize::{DeepSizeOf, Context};
            use core::mem;

            $(
                impl DeepSizeOf for $type {
                    fn deep_size_of_children(&self, _context: &mut Context) -> usize {
                        0
                    }

                    fn deep_size_of(&self) -> usize {
                        mem::size_of::<$type>()
                    }
                }
            )*

            #[test]
            fn deepsize_implementations() {
                $(
                    assert_eq!(
                        mem::size_of::<$type>(),
                        $type::try_from_usize(0).unwrap().deep_size_of(),
                    );
                )*
            }
        }
    };
}

// Implement `DeepSizeOf` when the `deepsize` feature is enabled
impl_deepsize! {
    Spur,
    MiniSpur,
    MicroSpur,
    LargeSpur,
}

macro_rules! impl_abomonation {
    ($($type:ident),* $(,)?) => {
        #[cfg(all(feature = "abomonation", not(feature = "no-std")))]
        mod __abomonation {
            use super::{$($type),*};
            #[cfg(test)]
            use super::Key;
            use abomonation::Abomonation;
            use std::io::{self, Write};

            $(
                impl Abomonation for $type {
                    unsafe fn entomb<W: Write>(&self, write: &mut W) -> io::Result<()> {
                        self.key.entomb(write)
                    }

                    unsafe fn exhume<'a, 'b>(&'a mut self, bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
                        self.key.exhume(bytes)
                    }

                    fn extent(&self) -> usize {
                        self.key.extent()
                    }
                }
            )*

            #[test]
            fn abomonation_implementations() {
                let mut buf = Vec::new();

                $(
                    unsafe {
                        let base = $type::try_from_usize(0).unwrap();

                        abomonation::encode(&base, &mut buf).unwrap();
                        assert_eq!(base, *abomonation::decode(&mut buf [..]).unwrap().0);
                    }

                    buf.clear();
                )*
            }
        }
    };
}

// Implement `Abomonation` when the `abomonation` feature is enabled
impl_abomonation! {
    Spur,
    MiniSpur,
    MicroSpur,
    LargeSpur,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large() {
        let zero = LargeSpur::try_from_usize(0).unwrap();
        let max = LargeSpur::try_from_usize(usize::max_value() - 1).unwrap();
        let default = LargeSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), usize::max_value() - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn large_max_returns_none() {
        assert_eq!(None, LargeSpur::try_from_usize(usize::max_value()));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn large_max_panics() {
        LargeSpur::try_from_usize(usize::max_value()).unwrap();
    }

    #[test]
    fn spur() {
        let zero = Spur::try_from_usize(0).unwrap();
        let max = Spur::try_from_usize(u32::max_value() as usize - 1).unwrap();
        let default = Spur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u32::max_value() as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn spur_returns_none() {
        assert_eq!(None, Spur::try_from_usize(u32::max_value() as usize));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn spur_panics() {
        Spur::try_from_usize(u32::max_value() as usize).unwrap();
    }

    #[test]
    fn mini() {
        let zero = MiniSpur::try_from_usize(0).unwrap();
        let max = MiniSpur::try_from_usize(u16::max_value() as usize - 1).unwrap();
        let default = MiniSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u16::max_value() as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn mini_returns_none() {
        assert_eq!(None, MiniSpur::try_from_usize(u16::max_value() as usize));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn mini_panics() {
        MiniSpur::try_from_usize(u16::max_value() as usize).unwrap();
    }

    #[test]
    fn micro() {
        let zero = MicroSpur::try_from_usize(0).unwrap();
        let max = MicroSpur::try_from_usize(u8::max_value() as usize - 1).unwrap();
        let default = MicroSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u8::max_value() as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn micro_returns_none() {
        assert_eq!(None, MicroSpur::try_from_usize(u8::max_value() as usize));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn micro_panics() {
        MicroSpur::try_from_usize(u8::max_value() as usize).unwrap();
    }

    #[test]
    #[cfg(feature = "serialize")]
    fn all_serialize() {
        let large = LargeSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&large).unwrap();

        let normal = Spur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&normal).unwrap();

        let mini = MiniSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&mini).unwrap();

        let micro = MicroSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&micro).unwrap();
    }
}
