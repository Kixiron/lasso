use core::marker::PhantomData;

#[macro_export]
#[doc(hidden)]
macro_rules! unique {
    ($name:ident) => {
        let tag = unsafe { $crate::unique::Tag::new() };
        let __guard;
        let $name = unsafe { $crate::unique::Unique::new(tag) };
        {
            if false {
                struct InnerTag<'unique>(&'unique $crate::unique::Tag<'unique>);

                impl<'id> ::core::ops::Drop for InnerTag<'id> {
                    fn drop(&mut self) {}
                }

                __guard = InnerTag(&tag);
            }
        }
    };
}

/// A unique, zero-sized lifetime that guarantees instance-unique access
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub struct Unique<'unique> {
    __tag: Tag<'unique>,
}

impl<'unique> Unique<'unique> {
    /// Do *not* use this function, use the `unique!()` macro
    #[doc(hidden)]
    pub unsafe fn new(__tag: Tag<'unique>) -> Unique<'unique> {
        Unique { __tag }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub struct Tag<'unique> {
    __phantom: PhantomData<&'unique mut &'unique fn(&'unique ()) -> &'unique ()>,
}

impl<'unique> Tag<'unique> {
    /// Do *not* use this function, use the `unique!()` macro
    #[doc(hidden)]
    pub unsafe fn new() -> Self {
        Tag {
            __phantom: PhantomData,
        }
    }
}

#[test]
#[allow(clippy::eq_op)]
fn unique_works() {
    unique!(a);
    unique!(b);

    assert_eq!(a, a);
    assert_eq!(b, b);
}

#[test]
fn unique_is_zst() {
    assert_eq!(core::mem::size_of::<Unique>(), 0);
}
