#[cfg(feature = "no-std")]
use alloc::string::String;
use core::{
    fmt::{self, Debug, Write},
    marker::PhantomData,
    num::{NonZeroU16, NonZeroU32, NonZeroU8, NonZeroUsize},
};

/// Types implementing this trait can be used as keys for all Rodeos
///
/// # Safety
///
/// into/from must be perfectly symmetrical, any key that goes on must be perfectly reproduced with the other
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
pub unsafe trait Key: Copy + Eq {
    /// Returns the `usize` that represents the current key
    fn into_usize(self) -> usize;

    /// Attempts to create a key from a `usize`, returning `None` if it fails
    fn try_from_usize(int: usize) -> Option<Self>;
}

/// A key type taking up `size_of::<usize>()` bytes of space (generally 4 or 8 bytes)
///
/// Internally is a `NonZeroUsize` to allow for space optimizations when stored inside of an [`Option`]
///
/// The type parameter `T` represents the type this key came from (e.g., `String`), providing
/// type safety to prevent using a key from one Rodeo with a different Rodeo.
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[repr(C)]
pub struct LargeSpur<T = String> {
    key: NonZeroUsize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for LargeSpur<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for LargeSpur<T> {}

impl<T> PartialEq for LargeSpur<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for LargeSpur<T> {}

impl<T> PartialOrd for LargeSpur<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for LargeSpur<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> core::hash::Hash for LargeSpur<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> LargeSpur<T> {
    /// Returns the [`NonZeroUsize`] backing the current `LargeSpur`
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn into_inner(self) -> NonZeroUsize {
        self.key
    }
}

unsafe impl<T> Key for LargeSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn into_usize(self) -> usize {
        self.key.get() - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[cfg_attr(feature = "inline-more", inline)]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < usize::MAX {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroUsize
            unsafe {
                Some(Self {
                    key: NonZeroUsize::new_unchecked(int + 1),
                    _marker: PhantomData,
                })
            }
        } else {
            None
        }
    }
}

impl<T> Default for LargeSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl<T> Debug for LargeSpur<T> {
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
/// The type parameter `T` represents the type this key came from (e.g., `String`), providing
/// type safety to prevent using a key from one Rodeo with a different Rodeo.
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[repr(C)]
pub struct Spur<T = String> {
    key: NonZeroU32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Spur<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Spur<T> {}

impl<T> PartialEq for Spur<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for Spur<T> {}

impl<T> PartialOrd for Spur<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Spur<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> core::hash::Hash for Spur<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> Spur<T> {
    /// Returns the [`NonZeroU32`] backing the current `Spur`
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn into_inner(self) -> NonZeroU32 {
        self.key
    }
}

unsafe impl<T> Key for Spur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u32::MAX - 1`
    #[cfg_attr(feature = "inline-more", inline)]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u32::MAX as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU32
            unsafe {
                Some(Self {
                    key: NonZeroU32::new_unchecked(int as u32 + 1),
                    _marker: PhantomData,
                })
            }
        } else {
            None
        }
    }
}

impl<T> Default for Spur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl<T> Debug for Spur<T> {
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
/// The type parameter `T` represents the type this key came from (e.g., `String`), providing
/// type safety to prevent using a key from one Rodeo with a different Rodeo.
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[repr(C)]
pub struct MiniSpur<T = String> {
    key: NonZeroU16,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for MiniSpur<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for MiniSpur<T> {}

impl<T> PartialEq for MiniSpur<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for MiniSpur<T> {}

impl<T> PartialOrd for MiniSpur<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for MiniSpur<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> core::hash::Hash for MiniSpur<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> MiniSpur<T> {
    /// Returns the [`NonZeroU16`] backing the current `MiniSpur`
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn into_inner(self) -> NonZeroU16 {
        self.key
    }
}

unsafe impl<T> Key for MiniSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u16::MAX - 1`
    #[cfg_attr(feature = "inline-more", inline)]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u16::MAX as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU16
            unsafe {
                Some(Self {
                    key: NonZeroU16::new_unchecked(int as u16 + 1),
                    _marker: PhantomData,
                })
            }
        } else {
            None
        }
    }
}

impl<T> Default for MiniSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl<T> Debug for MiniSpur<T> {
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
/// The type parameter `T` represents the type this key came from (e.g., `String`), providing
/// type safety to prevent using a key from one Rodeo with a different Rodeo.
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[repr(C)]
pub struct MicroSpur<T = String> {
    key: NonZeroU8,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for MicroSpur<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for MicroSpur<T> {}

impl<T> PartialEq for MicroSpur<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for MicroSpur<T> {}

impl<T> PartialOrd for MicroSpur<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for MicroSpur<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> core::hash::Hash for MicroSpur<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> MicroSpur<T> {
    /// Returns the [`NonZeroU8`] backing the current `MicroSpur`
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn into_inner(self) -> NonZeroU8 {
        self.key
    }
}

unsafe impl<T> Key for MicroSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `u8::MAX - 1`
    #[cfg_attr(feature = "inline-more", inline)]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u8::MAX as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU8
            unsafe {
                Some(Self {
                    key: NonZeroU8::new_unchecked(int as u8 + 1),
                    _marker: PhantomData,
                })
            }
        } else {
            None
        }
    }
}

impl<T> Default for MicroSpur<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::try_from_usize(0).unwrap()
    }
}

impl<T> Debug for MicroSpur<T> {
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
            use core::marker::PhantomData;
            use serde::{
                de::{Deserialize, Deserializer},
                ser::{Serialize, Serializer},
            };
            use core::num::{$($ty),*};

            $(
                impl<T> Serialize for $key<T> {
                    #[cfg_attr(feature = "inline-more", inline)]
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        self.key.serialize(serializer)
                    }
                }

                impl<'de, T> Deserialize<'de> for $key<T> {
                    #[cfg_attr(feature = "inline-more", inline)]
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let key = <$ty>::deserialize(deserializer)?;
                        Ok(Self { key, _marker: PhantomData })
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
            #[cfg(feature = "no-std")]
            use alloc::string::String;
            use super::{$($type),*};
            #[cfg(test)]
            use super::Key;
            use deepsize::{DeepSizeOf, Context};
            use core::mem;

            $(
                impl<T> DeepSizeOf for $type<T> {
                    fn deep_size_of_children(&self, _context: &mut Context) -> usize {
                        0
                    }

                    fn deep_size_of(&self) -> usize {
                        mem::size_of::<$type<T>>()
                    }
                }
            )*

            #[test]
            fn deepsize_implementations() {
                $(
                    assert_eq!(
                        mem::size_of::<$type<String>>(),
                        <$type<String>>::try_from_usize(0).unwrap().deep_size_of(),
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
                impl<T> Abomonation for $type<T> {
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
                        let base = <$type<String>>::try_from_usize(0).unwrap();

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
    #[cfg(feature = "no-std")]
    use alloc::string::String;

    #[test]
    fn large() {
        let zero: LargeSpur<String> = LargeSpur::try_from_usize(0).unwrap();
        let max: LargeSpur<String> = LargeSpur::try_from_usize(usize::MAX - 1).unwrap();
        let default: LargeSpur<String> = LargeSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), usize::MAX - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn large_max_returns_none() {
        assert_eq!(
            None::<LargeSpur<String>>,
            LargeSpur::try_from_usize(usize::MAX)
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn large_max_panics() {
        <LargeSpur<String>>::try_from_usize(usize::MAX).unwrap();
    }

    #[test]
    fn spur() {
        let zero: Spur<String> = Spur::try_from_usize(0).unwrap();
        let max: Spur<String> = Spur::try_from_usize(u32::MAX as usize - 1).unwrap();
        let default: Spur<String> = Spur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u32::MAX as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn spur_returns_none() {
        assert_eq!(
            None::<Spur<String>>,
            Spur::try_from_usize(u32::MAX as usize)
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn spur_panics() {
        <Spur<String>>::try_from_usize(u32::MAX as usize).unwrap();
    }

    #[test]
    fn mini() {
        let zero: MiniSpur<String> = MiniSpur::try_from_usize(0).unwrap();
        let max: MiniSpur<String> = MiniSpur::try_from_usize(u16::MAX as usize - 1).unwrap();
        let default: MiniSpur<String> = MiniSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u16::MAX as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn mini_returns_none() {
        assert_eq!(
            None::<MiniSpur<String>>,
            MiniSpur::try_from_usize(u16::MAX as usize)
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn mini_panics() {
        <MiniSpur<String>>::try_from_usize(u16::MAX as usize).unwrap();
    }

    #[test]
    fn micro() {
        let zero: MicroSpur<String> = MicroSpur::try_from_usize(0).unwrap();
        let max: MicroSpur<String> = MicroSpur::try_from_usize(u8::MAX as usize - 1).unwrap();
        let default: MicroSpur<String> = MicroSpur::default();

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u8::MAX as usize - 1);
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn micro_returns_none() {
        assert_eq!(
            None::<MicroSpur<String>>,
            MicroSpur::try_from_usize(u8::MAX as usize)
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn micro_panics() {
        <MicroSpur<String>>::try_from_usize(u8::MAX as usize).unwrap();
    }

    #[test]
    #[cfg(feature = "serialize")]
    fn all_serialize() {
        let large: LargeSpur<String> = LargeSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&large).unwrap();

        let normal: Spur<String> = Spur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&normal).unwrap();

        let mini: MiniSpur<String> = MiniSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&mini).unwrap();

        let micro: MicroSpur<String> = MicroSpur::try_from_usize(0).unwrap();
        let _ = serde_json::to_string(&micro).unwrap();
    }

    /// Ensure that `Option<Key>` has the same size as `Key` (niche optimization)
    #[test]
    fn option_niche_optimization() {
        use core::mem::size_of;

        // Option<Spur> should be the same size as Spur due to NonZero niche
        assert_eq!(size_of::<Spur<String>>(), size_of::<Option<Spur<String>>>());
        assert_eq!(
            size_of::<MiniSpur<String>>(),
            size_of::<Option<MiniSpur<String>>>()
        );
        assert_eq!(
            size_of::<MicroSpur<String>>(),
            size_of::<Option<MicroSpur<String>>>()
        );
        assert_eq!(
            size_of::<LargeSpur<String>>(),
            size_of::<Option<LargeSpur<String>>>()
        );
    }
}
