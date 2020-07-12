use core::num::{NonZeroU16, NonZeroU32, NonZeroU8, NonZeroUsize};
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// Types implementing this trait can be used as keys for all Rodeos
///
/// # Safety
///
/// into/from must be perfectly symmetrical, any key that goes on must be perfectly reproduced with the other
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
pub unsafe trait Key: Copy + Eq {
    /// Returns the `usize` that represents the current key
    ///
    /// # Safety
    ///
    /// To be safe, `into_usize` and `try_from_usize` must be symmetrical, meaning that any usize given
    /// to `into_usize` must be the same after going through `try_from_usize`
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
#[cfg_attr(feature = "serialize", derive(Deserialize, Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LargeSpur {
    key: NonZeroUsize,
}

unsafe impl Key for LargeSpur {
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

/// The default key for every Rodeo, uses only 32bits of space
///
/// Internally is a `NonZeroU32` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[cfg_attr(feature = "serialize", derive(Deserialize, Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Spur {
    key: NonZeroU32,
}

unsafe impl Key for Spur {
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

/// A miniature Key utilizing only 16 bits of space
///
/// Internally is a `NonZeroU16` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[cfg_attr(feature = "serialize", derive(Deserialize, Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MiniSpur {
    key: NonZeroU16,
}

unsafe impl Key for MiniSpur {
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

/// A miniature Key utilizing only 8 bits of space
///
/// Internally is a `NonZeroU8` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
#[cfg_attr(feature = "serialize", derive(Deserialize, Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MicroSpur {
    key: NonZeroU8,
}

unsafe impl Key for MicroSpur {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large() {
        let zero = LargeSpur::try_from_usize(0).unwrap();
        let max = LargeSpur::try_from_usize(usize::max_value() - 1).unwrap();
        let default = LargeSpur::default();

        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), usize::max_value() - 1);
            assert_eq!(default.into_usize(), 0);
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
        let default = Spur::default();

        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u32::max_value() as usize - 1);
            assert_eq!(default.into_usize(), 0);
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
        let default = MiniSpur::default();

        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u16::max_value() as usize - 1);
            assert_eq!(default.into_usize(), 0);
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
        let default = MicroSpur::default();

        unsafe {
            assert_eq!(zero.into_usize(), 0);
            assert_eq!(max.into_usize(), u8::max_value() as usize - 1);
            assert_eq!(default.into_usize(), 0);
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
