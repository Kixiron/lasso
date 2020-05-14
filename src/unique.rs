use core::marker::PhantomData;

/// A unique, zero-sized lifetime that guarantees instance-unique access
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Unique<'unique> {
    __lifetime: PhantomData<&'unique mut &'unique ()>,
}

#[test]
fn unique_is_zst() {
    assert_eq!(core::mem::size_of::<Unique>(), 0);
}
