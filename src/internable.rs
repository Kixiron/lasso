use core::{
    hash::Hash,
    mem::{align_of, MaybeUninit},
    num::NonZeroUsize,
    slice,
};

// FIXME: Use `pointer::is_aligned_to()` once rust/#96284 is stabilized
fn is_aligned_to(ptr: *const u8, align: usize) -> bool {
    debug_assert!(align.is_power_of_two());
    ptr as usize % align == 0
}

/// Trait for types that can be interned within an interner
///
/// # Safety
///
pub unsafe trait Internable: Hash + Eq + AsRef<Self> {
    /// The alignment of the internable type
    const ALIGNMENT: NonZeroUsize;

    /// A reference to an empty instance of this type
    fn empty() -> &'static Self;

    /// Gets the length in bytes of the encoded item
    ///
    /// Note that a byte length of greater than zero doesn't necessarily mean
    /// that the current internable is empty, use [`Internable::is_empty()`]
    /// to check for that
    fn byte_len(&self) -> usize;

    /// Copies the current internable to an uninitalized slice of data,
    /// returning a reference to the newly initialized data
    ///
    /// # Safety
    ///
    unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self;

    /// Check whether the current internable is empty
    ///
    /// Note that even if `.is_empty()` returns true, that doesn't imply that
    /// [`Internable::byte_len()`] is equal to zero
    #[inline]
    fn is_empty(&self) -> bool {
        self.byte_len() == 0
    }
}

unsafe impl Internable for [u8] {
    const ALIGNMENT: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(align_of::<u8>()) };

    #[inline]
    fn empty() -> &'static Self {
        &[]
    }

    #[inline]
    fn byte_len(&self) -> usize {
        self.len()
    }

    #[inline]
    unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
        let length = self.byte_len();

        debug_assert_eq!(dest.len(), length);
        debug_assert!(is_aligned_to(dest.as_ptr().cast(), Self::ALIGNMENT.get()));

        // FIXME: `MaybeUninit::write_slice()` once rust/#79995 lands
        unsafe {
            let (this, dest) = (self.as_ptr(), dest.as_mut_ptr().cast::<u8>());
            dest.copy_from_nonoverlapping(this, length);

            // Safety: We've initialized all bytes of the slice
            slice::from_raw_parts(dest, length)
        }
    }
}

unsafe impl Internable for str {
    const ALIGNMENT: NonZeroUsize = <[u8] as Internable>::ALIGNMENT;

    #[inline]
    fn empty() -> &'static Self {
        ""
    }

    #[inline]
    fn byte_len(&self) -> usize {
        Internable::byte_len(self.as_bytes())
    }

    #[inline]
    unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
        unsafe {
            let bytes = Internable::copy_to_slice(self.as_bytes(), dest);

            // Safety: We know our slice is valid utf8
            core::str::from_utf8_unchecked(bytes)
        }
    }
}

#[cfg(all(not(feature = "no-std"), feature = "std-ffi"))]
mod std_ffi {
    use super::Internable;
    use core::{mem::MaybeUninit, num::NonZeroUsize};
    use std::{ffi::OsStr, path::Path};

    #[cfg(target_family = "unix")]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    #[cfg(not(any(target_family = "unix", target_family = "windows", target_os = "wasi")))]
    compile_error!(
        "Currently std::ffi::OsStrExt is only available on the unix and windows \
        target families as well as the wasi target os.\n\
        To disable std::path::Path and std::ffi::OsStr support, please disable \
        the `std-ffi` feature on the `lasso` crate.",
    );

    /// # Warning
    ///
    /// On Windows due to there being no sound way to convert an `OsStr` into
    /// a byte slice (to and from or any other sort of slice without incurring an
    /// allocation) *on windows* these functions will panic on non-utf8 inputs
    ///
    unsafe impl Internable for Path {
        const ALIGNMENT: NonZeroUsize = <OsStr as Internable>::ALIGNMENT;

        #[inline]
        fn empty() -> &'static Self {
            Self::new("")
        }

        #[inline]
        fn byte_len(&self) -> usize {
            Internable::byte_len(self.as_os_str())
        }

        #[inline]
        unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
            unsafe { Self::new(Internable::copy_to_slice(self.as_os_str(), dest)) }
        }
    }

    /// # Warning
    ///
    /// On Windows due to there being no sound way to convert an `OsStr` into
    /// a byte slice (to and from or any other sort of slice without incurring an
    /// allocation) *on windows* these functions will panic on non-utf8 inputs
    ///
    #[cfg(any(target_family = "unix", target_os = "wasi"))]
    unsafe impl Internable for OsStr {
        const ALIGNMENT: usize = align_of::<u8>();

        #[inline]
        fn empty() -> &'static Self {
            Self::new("")
        }

        #[inline]
        fn byte_len(&self) -> usize {
            Internable::byte_len(self.as_bytes())
        }

        #[inline]
        unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
            let bytes = unsafe { Internable::copy_to_slice(self.as_bytes(), dest) };
            Self::from_bytes(bytes)
        }
    }

    /// # Warning
    ///
    /// On Windows due to there being no sound way to convert an `OsStr` into
    /// a byte slice (to and from or any other sort of slice without incurring an
    /// allocation) *on windows* these functions will panic on non-utf8 inputs
    ///
    // FIXME: Figure out a better method of losslessly converting an OsStr
    //        to a byte sequence, probably depends on rust/#95290
    #[cfg(target_family = "windows")]
    unsafe impl Internable for OsStr {
        const ALIGNMENT: NonZeroUsize = <str as Internable>::ALIGNMENT;

        #[inline]
        fn empty() -> &'static Self {
            Self::new("")
        }

        #[inline]
        fn byte_len(&self) -> usize {
            let string = self
                .to_str()
                .expect("attempted to intern a non-utf8 OsStr on windows");
            Internable::byte_len(string)
        }

        #[inline]
        unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
            let string = self
                .to_str()
                .expect("attempted to intern a non-utf8 OsStr on windows");
            let bytes = unsafe { Internable::copy_to_slice(string, dest) };
            Self::new(bytes)
        }
    }
}

#[cfg(not(feature = "no-std"))]
mod cstr {
    use super::Internable;
    use core::{mem::MaybeUninit, num::NonZeroUsize};
    use std::ffi::CStr;

    unsafe impl Internable for CStr {
        const ALIGNMENT: NonZeroUsize = <[u8] as Internable>::ALIGNMENT;

        #[inline]
        fn empty() -> &'static Self {
            // Safety: The string we give is null-terminated
            unsafe { CStr::from_bytes_with_nul_unchecked(b"\0") }
        }

        // We re-implement `.is_empty()` because the length of an empty CStr
        // is 1 because of its null terminator, but we only actually want
        // to intern strings with an actual payload to them besides the null byte
        #[inline]
        fn is_empty(&self) -> bool {
            self.to_bytes().len() == 0
        }

        #[inline]
        fn byte_len(&self) -> usize {
            self.to_bytes_with_nul().len()
        }

        #[inline]
        unsafe fn copy_to_slice<'a>(&self, dest: &'a mut [MaybeUninit<u8>]) -> &'a Self {
            let bytes = unsafe { Internable::copy_to_slice(self.to_bytes_with_nul(), dest) };
            debug_assert!(Self::from_bytes_with_nul(bytes).is_ok());

            // Safety: The bytes we copied into `dest` are null-terminated
            unsafe { Self::from_bytes_with_nul_unchecked(bytes) }
        }
    }
}
