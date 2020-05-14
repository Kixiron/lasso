use crate::unique::Unique;

use core::num::{NonZeroU16, NonZeroU32, NonZeroU8, NonZeroUsize};

/// Types implementing this trait can be used as keys for all Rodeos
///
/// # Safety
///
/// into/from must be perfectly symmetrical, any key that goes on must be perfectly reproduced with the other
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
pub unsafe trait Key<'unique>: Copy + Eq {
    /// Returns the `usize` that represents the current key
    ///
    /// # Safety
    ///
    /// To be safe, `into_usize` and `{try}_from_usize` must be symmetrical, meaning that any usize given
    /// to `into_usize` must be the same after going through `{try}_from_usize`
    ///
    unsafe fn into_usize(self) -> usize;

    /// Attempts to create a key from a `usize`, returning `None` if it fails
    fn try_from_usize(int: usize) -> Option<Self>;
}

/// The default key for every Rodeo, the same size as a `usize`
///
/// Internally is a `NonZeroUsize` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LargeSpur<'unique> {
    key: NonZeroUsize,
    unique: Unique<'unique>,
}

unsafe impl<'unique> Key<'unique> for LargeSpur<'unique> {
    #[inline]
    unsafe fn into_usize(self) -> usize {
        self.key.get() - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < usize::max_value() {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroUsize
            unsafe {
                Some(Self {
                    key: NonZeroUsize::new_unchecked(int + 1),
                    unique: Default::default(),
                })
            }
        } else {
            None
        }
    }
}

impl<'unique> Default for LargeSpur<'unique> {
    #[inline]
    fn default() -> Self {
        Self {
            // Safety: 1 is not 0
            key: unsafe { NonZeroUsize::new_unchecked(1) },
            unique: Default::default(),
        }
    }
}

/// The default key for every Rodeo, uses only 32bits of space
///
/// Internally is a `NonZeroU32` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Spur<'unique> {
    key: NonZeroU32,
    unique: Unique<'unique>,
}

unsafe impl<'unique> Key<'unique> for Spur<'unique> {
    #[inline]
    unsafe fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u32::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU32
            unsafe {
                Some(Self {
                    key: NonZeroU32::new_unchecked(int as u32 + 1),
                    unique: Default::default(),
                })
            }
        } else {
            None
        }
    }
}

impl<'unique> Default for Spur<'unique> {
    #[inline]
    fn default() -> Self {
        Self {
            // Safety: 1 is not 0
            key: unsafe { NonZeroU32::new_unchecked(1) },
            unique: Default::default(),
        }
    }
}

/// A miniature Key utilizing only 16 bits of space
///
/// Internally is a `NonZeroU16` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MiniSpur<'unique> {
    key: NonZeroU16,
    unique: Unique<'unique>,
}

unsafe impl<'unique> Key<'unique> for MiniSpur<'unique> {
    #[inline]
    unsafe fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u16::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU16
            unsafe {
                Some(Self {
                    key: NonZeroU16::new_unchecked(int as u16 + 1),
                    unique: Default::default(),
                })
            }
        } else {
            None
        }
    }
}

impl<'unique> Default for MiniSpur<'unique> {
    #[inline]
    fn default() -> Self {
        Self {
            // Safety: 1 is not 0
            key: unsafe { NonZeroU16::new_unchecked(1) },
            unique: Default::default(),
        }
    }
}

/// A miniature Key utilizing only 8 bits of space
///
/// Internally is a `NonZeroU8` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MicroSpur<'unique> {
    key: NonZeroU8,
    unique: Unique<'unique>,
}

unsafe impl<'unique> Key<'unique> for MicroSpur<'unique> {
    #[inline]
    unsafe fn into_usize(self) -> usize {
        self.key.get() as usize - 1
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u8::max_value() as usize {
            // Safety: The integer is less than the max value and then incremented by one, meaning that
            // is is impossible for a zero to inhabit the NonZeroU16
            unsafe {
                Some(Self {
                    key: NonZeroU8::new_unchecked(int as u8 + 1),
                    unique: Default::default(),
                })
            }
        } else {
            None
        }
    }
}

impl<'unique> Default for MicroSpur<'unique> {
    #[inline]
    fn default() -> Self {
        Self {
            // Safety: 1 is not 0
            key: unsafe { NonZeroU8::new_unchecked(1) },
            unique: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large() {
        let zero = LargeSpur::try_from_usize(0).unwrap();
        let max = LargeSpur::try_from_usize(usize::max_value() - 1).unwrap();

        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), usize::max_value() - 1);
        }
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
        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u32::max_value() as usize - 1);
        }
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
        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u16::max_value() as usize - 1);
        }
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
        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u8::max_value() as usize - 1);
        }
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
}
