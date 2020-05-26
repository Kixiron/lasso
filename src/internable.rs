use core::hash::Hash;

/// Represents something that's able to be interned
pub trait Internable: Hash + Eq + 'static {
    /// The raw data that is stored in the interner
    type Raw: Sized + Clone;

    /// Converts an `Internable` thing into raw data
    fn to_raw(&self) -> &[Self::Raw];

    /// Converts raw data into an `Internable`
    ///
    /// # Safety
    ///
    /// Operates off of raw data and must not mutate it in any way
    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self;
}

impl Internable for str {
    type Raw = u8;

    fn to_raw(&self) -> &[Self::Raw] {
        self.as_bytes()
    }

    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
        core::str::from_utf8_unchecked(raw)
    }
}

#[cfg(not(feature = "no-std"))]
impl Internable for std::ffi::CStr {
    type Raw = u8;

    fn to_raw(&self) -> &[Self::Raw] {
        self.to_bytes_with_nul()
    }

    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
        std::ffi::CStr::from_bytes_with_nul(raw).unwrap()
    }
}

impl<T> Internable for [T]
where
    T: Hash + Eq + 'static + Sized + Clone,
{
    type Raw = T;

    fn to_raw(&self) -> &[Self::Raw] {
        self
    }

    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
        raw
    }
}

// TODO: It *should* be possible to use this with arbitrary types, but as of now,
//       it produces hellish errors
// impl<T> Internable for T
// where
//     T: Hash + Eq + 'static + Sized + Clone,
// {
//     type Raw = Self;
//
//     fn to_raw(&self) -> &[Self::Raw] {
//         core::slice::from_ref(self)
//     }
//
//     unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
//         &raw[0]
//     }
// }
//
// impl<T> Internable for [T]
// where
//     T: Internable + Clone,
// {
//     type Raw = T;
//
//     fn to_raw(&self) -> &[Self::Raw] {
//         self
//     }
//
//     unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
//         raw
//     }
// }
