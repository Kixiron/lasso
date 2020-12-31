#![cfg_attr(feature = "no-std", no_std)]

use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice, str,
};
use lasso::{Key, LassoError, LassoErrorKind, Rodeo, Spur};
use libc::c_char;

/// The value of an invalid `key`, used to tell if a function has returned
/// a key or nothing
///
/// ```c
/// #include <stdio.h>
/// #include <string.h>
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///     
///     const char *string = "un-interned string!";
///     uint32_t key = lasso_rodeo_get(rodeo, string, strlen(string));
///     if (key == LASSO_INVALID_KEY) {
///         printf("%s has no key!\n", string);
///     } else {
///         printf("%s has the key %u\n", key);
///     }
/// }
/// ```
pub const LASSO_INVALID_KEY: u32 = 0;

/// Create a new [`Rodeo`] with the default settings
///
/// Uses a [`Spur`] (`uint32_t`) as the key type and Rust's [`RandomState`] as the underlying hasher
///
/// ```c
/// #include "lasso.h"
///
/// int main() {
///     // Create a new interner
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
/// [`RandomState`]: std::collections::hash_map::RandomState
#[no_mangle]
pub extern "C" fn lasso_rodeo_new() -> NonNull<OpaqueRodeo> {
    let rodeo = Box::into_raw(Box::new(OpaqueRodeo(Rodeo::new())));
    #[cfg(debug_assertions)]
    debug_assert!(!rodeo.is_null());

    // Safety: The pointer returned from `Box::into_raw` will never be null
    unsafe { NonNull::new_unchecked(rodeo) }
}

/// Dispose of a previously created [`Rodeo`]
///
/// # Safety
///
/// - `rodeo` must be non-null and must have previously been created by [`lasso_rodeo_new()`]
///   and must not yet have been disposed of
///
/// ```c
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     // Dispose of the interner
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
#[no_mangle]
pub unsafe extern "C" fn lasso_rodeo_dispose(rodeo: NonNull<OpaqueRodeo>) {
    let rodeo = Box::from_raw(rodeo.as_ptr());
    drop(rodeo);
}

/// Get the key value of a string, returning [`LASSO_INVALID_KEY`] if it doesn't exist
///
/// # Safety
///
/// - `rodeo` must be non-null and not currently mutably borrowed.
/// - `string` must be non-null and point to an array of utf8-encoded bytes of `length` length.
///
/// ```c
/// #include <assert.h>
/// #include <stdint.h>
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     char *some_random_string = "Any string you can imagine!";
///
///     // Attempt to get the key for a string
///     uint32_t key = lasso_rodeo_get(
///         rodeo,
///         some_random_string,
///         strlen(some_random_string),
///     );
///
///     // `lasso_rodeo_get()` returns `LASSO_INVALID_KEY` if the string hasn't
///     // been interned yet, so this call will succeed since the string we tried
///     // to get hasn't been interned
///     assert(key == LASSO_INVALID_KEY);
///
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
/// ```c
/// #include <assert.h>
/// #include <stdint.h>
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     char *some_random_string = "Any string you can imagine!";
///
///     // Intern the string so the `lasso_rodeo_get()` call succeeds
///     LassoErrorKind error = LASSO_ERROR_NONE;
///     lasso_rodeo_get_or_intern(
///         rodeo,
///         some_random_string,
///         strlen(some_random_string),
///         &error,
///     );
///     assert(error == LASSO_ERROR_NONE);
///
///     // Attempt to get the key for a string
///     uint32_t key = lasso_rodeo_get(
///         rodeo,
///         some_random_string,
///         strlen(some_random_string),
///     );
///
///     // `lasso_rodeo_get()` returns `LASSO_INVALID_KEY` if the string hasn't
///     // been interned yet, so this call will succeed since the string we tried
///     // to get has been interned
///     assert(key != LASSO_INVALID_KEY);
///
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
#[no_mangle]
pub unsafe extern "C" fn lasso_rodeo_get(
    rodeo: NonNull<OpaqueRodeo>,
    string: NonNull<c_char>,
    length: u64,
) -> u32 {
    let rodeo = rodeo.as_ref();

    // Turn the given char pointer and length into a `str`
    let string = string_from_raw_parts(string, length);

    if let Some(key) = rodeo.get(string) {
        #[cfg(debug_assertions)]
        debug_assert_ne!(key.into_usize() as u32, LASSO_INVALID_KEY);

        key.into_usize() as u32
    } else {
        LASSO_INVALID_KEY
    }
}

/// Get a string if it's been previously interned or intern it if it does not yet exist
///
/// If an error occurs while attempting to intern a string then [`LASSO_INVALID_KEY`]
/// will be returned and the pointed-to value within `error` will be populated with
/// the error code as a [`FFILassoErrorKind`]. If no error occurs then the pointed-to
/// value will be populated with [`FFILassoErrorKind::None`] and the returned value
/// will be the key associated with the given string
///
/// # Safety
///
/// - `rodeo` must be non-null and not currently mutably borrowed.
/// - `string` must be non-null and point to an array of utf8-encoded bytes of `length` length.
/// - `error` must be a non-null pointer to a `u8`
///
/// ```c
/// #include <assert.h>
/// #include <stdint.h>
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     char *some_random_string = "Any string you can imagine!";
///
///     // Create a variable to hold the error if one occurs
///     LassoErrorKind error = LASSO_ERROR_NONE;
///
///     // Intern the string if it doesn't exist and return the key assigned to it
///     uint32_t key = lasso_rodeo_get_or_intern(
///         rodeo,
///         some_random_string,
///         strlen(some_random_string),
///         &error,
///     );
///     
///     // If everything went smoothly, the returned `key` will have the key associated
///     // with the given string and `error` will hold `LASSO_ERROR_NONE`
///     if (error == LASSO_ERROR_NONE) {
///         assert(key != LASSO_INVALID_KEY);
///     }
///     // If the intern call succeeds, `error` will hold the error that occurred
///     // and the returned key will be `LASSO_INVALID_KEY`
///     else {
///         assert(key == LASSO_INVALID_KEY);
///     }
///
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
#[no_mangle]
pub unsafe extern "C" fn lasso_rodeo_get_or_intern(
    mut rodeo: NonNull<OpaqueRodeo>,
    string: NonNull<c_char>,
    length: u64,
    mut error: NonNull<FFILassoErrorKind>,
) -> u32 {
    let rodeo = rodeo.as_mut();
    let error = error.as_mut();

    // Turn the given char pointer and length into a `str`
    let string = string_from_raw_parts(string, length);

    match rodeo.try_get_or_intern(string) {
        Ok(key) => {
            #[cfg(debug_assertions)]
            debug_assert_ne!(key.into_usize() as u32, LASSO_INVALID_KEY);
            *error = FFILassoErrorKind::None;

            key.into_usize() as u32
        }

        Err(lasso_error) => {
            let error_kind = lasso_error.into();
            *error = error_kind;

            LASSO_INVALID_KEY
        }
    }
}

/// Resolve a key into a string, returning an [`None`] if one occurs
///
/// `length` will be set to the length of the string in utf8 encoded bytes
/// and the return value will be null if the key does not exist in the current
/// interner
///
/// # Safety
///
/// - `rodeo` must be non-null and not currently mutably borrowed.
///
/// ```c
/// #include <assert.h>
/// #include <stdio.h>
/// #include <stdint.h>
/// #include "lasso.h"
///
/// int main() {
///     Rodeo *rodeo = lasso_rodeo_new();
///
///     char *some_random_string = "Any string you can imagine!";
///
///     // Intern the string
///     LassoErrorKind error = LASSO_ERROR_NONE;
///     uint32_t key = lasso_rodeo_get_or_intern(
///         rodeo,
///         some_random_string,
///         strlen(some_random_string),
///         &error,
///     );
///     assert(key != LASSO_INVALID_KEY);
///
///     // Make a variable to hold the string's length
///     uint64_t length = 0;
///
///     // Resolve the key for the underlying string
///     char *ptr = lasso_rodeo_resolve(rodeo, key, &length);
///
///     // If the key is found in the current interner, `ptr` will hold the pointer to
///     // the string and `length` will hold the length of the string in utf8 encoded
///     // bytes
///     if (ptr) {
///         printf("The key %u resolved to the string ", key);
///         // We use `fwrite()` to print out the string since utf8 allows interior null bytes
///         fwrite(string, sizeof(char), length, stdout);
///         printf("\n");
///     }
///     // If the key doesn't exist in the current interner, `ptr` will be null
///     else {
///         printf("The key %u could not be found!\n", key);
///     }
///
///     lasso_rodeo_dispose(rodeo);
/// }
/// ```
///
#[no_mangle]
pub unsafe extern "C" fn lasso_rodeo_resolve(
    rodeo: NonNull<OpaqueRodeo>,
    key: u32,
    mut length: NonNull<u64>,
) -> Option<NonNull<c_char>> {
    let rodeo = rodeo.as_ref();
    let key = Spur::try_from_usize(key as usize)?;

    let string = rodeo.try_resolve(&key)?;
    *length.as_mut() = string.len() as u64;

    Some(NonNull::new_unchecked(string.as_ptr() as *mut c_char))
}

/// Turns a non-null pointer to utf8 encoded characters and a length into a Rust
/// string slice
unsafe fn string_from_raw_parts<'a>(string: NonNull<c_char>, length: u64) -> &'a str {
    let slice = slice::from_raw_parts(string.as_ptr() as *const u8, length as usize);
    #[cfg(debug_assertions)]
    debug_assert!(str::from_utf8(slice).is_ok());

    str::from_utf8_unchecked(slice)
}

/// A shim struct to allow passing [`Rodeo`]s across the FFI boundary
/// as opaque pointers
pub struct OpaqueRodeo(pub Rodeo);

impl Deref for OpaqueRodeo {
    type Target = Rodeo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OpaqueRodeo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// An error that's safe to pass across the FFI boundary
#[repr(u8)]
pub enum FFILassoErrorKind {
    /// No error occurred
    None = 0,
    /// A memory limit set using [`MemoryLimits`] was reached, and no more memory could be allocated
    ///
    /// [`MemoryLimits`]: lasso::MemoryLimits
    MemoryLimitReached = 1,
    /// A [`Key`] implementation returned [`None`], meaning it could not produce any more keys
    KeySpaceExhaustion = 2,
    /// A memory allocation failed
    FailedAllocation = 3,
}

impl From<LassoError> for FFILassoErrorKind {
    fn from(error: LassoError) -> Self {
        error.kind().into()
    }
}

impl From<LassoErrorKind> for FFILassoErrorKind {
    fn from(kind: LassoErrorKind) -> Self {
        match kind {
            LassoErrorKind::MemoryLimitReached => Self::MemoryLimitReached,
            LassoErrorKind::KeySpaceExhaustion => Self::KeySpaceExhaustion,
            LassoErrorKind::FailedAllocation => Self::FailedAllocation,
        }
    }
}

/// No error occurred
pub const LASSO_ERROR_NONE: u8 = 0; // FFILassoErrorKind::None as u8

/// A memory limit set using [`MemoryLimits`] was reached, and no more memory could be allocated
///
/// [`MemoryLimits`]: lasso::MemoryLimits
pub const LASSO_ERROR_MEMORY_LIMIT_REACHED: u8 = 1; // FFILassoErrorKind::MemoryLimitReached as u8

/// A [`Key`] implementation returned [`None`], meaning it could not produce any more keys
pub const LASSO_ERROR_KEY_SPACE_EXHAUSTION: u8 = 2; // FFILassoErrorKind::KeySpaceExhaustion as u8

/// A memory allocation failed
pub const LASSO_ERROR_FAILED_ALLOCATION: u8 = 3; // FFILassoErrorKind::FailedAllocation as u8
