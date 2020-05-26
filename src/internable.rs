use core::hash::Hash;

/// Represents something that's able to be interned
pub trait Internable: Hash + Eq + AsRef<Self> + 'static {
    /// The raw data that is stored in the interner
    type Raw: Sized;

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

impl<T> Internable for T
where
    T: Hash + Eq + AsRef<Self> + 'static + Sized,
{
    type Raw = Self;

    fn to_raw(&self) -> &[Self::Raw] {
        core::slice::from_ref(self)
    }

    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
        &raw[0]
    }
}

impl<T> Internable for [T]
where
    T: Internable,
{
    type Raw = T;

    fn to_raw(&self) -> &[Self::Raw] {
        self
    }

    unsafe fn from_raw(raw: &[Self::Raw]) -> &Self {
        raw
    }
}
