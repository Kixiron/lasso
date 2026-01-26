use crate::{rodeo::Internable, rodeo::InternableRef, LassoError, LassoErrorKind, LassoResult};
use alloc::alloc::{alloc, dealloc, Layout};
use core::{marker::PhantomData, mem::size_of, num::NonZeroUsize, ptr::NonNull};

/// A bucket to hold a number of stored items
pub(super) struct Bucket<T: Internable> {
    /// The start of uninitialized memory within `items`
    index: usize,
    /// A pointer to the start of the data
    items: NonNull<u8>,
    /// The total number of bytes that can be stored
    capacity: NonZeroUsize,
    /// Marker for the internable type
    _marker: PhantomData<T>,
}

impl<T: Internable> Bucket<T> {
    const ALIGN: usize = T::Ref::ALIGNMENT;

    /// Allocates a bucket with space for `capacity` bytes
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> LassoResult<Self> {
        const { assert!(T::Ref::ALIGNMENT.is_power_of_two(), "alignment must be a power of two") };

        unsafe {
            debug_assert!(
                Layout::from_size_align(size_of::<u8>() * capacity.get(), Self::ALIGN).is_ok()
            );

            // Safety: ALIGN is a non-zero power of two and the
            //         size will not overflow when rounded up
            let layout =
                Layout::from_size_align_unchecked(size_of::<u8>() * capacity.get(), Self::ALIGN);

            // Allocate the bucket's memory
            let items = NonNull::new(alloc(layout))
                // TODO: When `Result`s are piped through return this as a unique error
                .ok_or_else(|| LassoError::new(LassoErrorKind::FailedAllocation))?
                .cast();

            Ok(Self {
                index: 0,
                capacity,
                items,
                _marker: PhantomData,
            })
        }
    }

    /// Get the number of available slots for the current bucket
    pub(crate) fn free_elements(&self) -> usize {
        self.capacity.get() - self.index
    }

    /// Returns whether the current bucket is full
    pub(crate) fn is_full(&self) -> bool {
        self.index == self.capacity.get()
    }

    /// Marks the bucket as being totally unused, meaning that all of `capacity`
    /// is valid for allocations
    pub(crate) fn clear(&mut self) {
        self.index = 0;
    }

    /// Push a value to the current bucket, returning a pointer to it
    ///
    /// # Safety
    ///
    /// The current bucket must have room for all bytes of the value (including alignment padding)
    /// and the caller promises to forget the reference before the arena is dropped.
    ///
    pub(crate) unsafe fn push_slice(&mut self, value: &T::Ref) -> &'static T::Ref {
        debug_assert!(!self.is_full());

        let slice = value.as_bytes();
        let count = value.len();

        // Align the index to the required alignment for T::Ref
        let aligned_index = (self.index + Self::ALIGN - 1) & !(Self::ALIGN - 1);

        debug_assert!(aligned_index + slice.len() <= self.capacity.get());

        unsafe {
            // Get a pointer to the aligned start of free bytes
            let ptr = self.items.as_ptr().add(aligned_index);

            // Copy the data from the source into the bucket's buffer
            ptr.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
            // Increment the index so that the data we just added isn't overwritten
            self.index = aligned_index + slice.len();

            // Create a reference from the allocated data
            // Safety: The source data was valid, so the created buffer will be as well.
            // The pointer is properly aligned because we aligned the index above.
            T::Ref::from_raw_parts(ptr, count)
        }
    }

    /// Returns the space needed to store `len` bytes with the given alignment,
    /// accounting for any padding needed.
    pub(crate) fn space_needed(&self, len: usize) -> usize {
        let aligned_index = (self.index + Self::ALIGN - 1) & !(Self::ALIGN - 1);
        let padding = aligned_index - self.index;
        padding + len
    }
}

impl<T: Internable> Drop for Bucket<T> {
    fn drop(&mut self) {
        // Safety: We have exclusive access to the pointers since the contract of
        //         `store_str` should be withheld
        unsafe {
            let items = self.items.as_ptr();

            debug_assert!(Layout::from_size_align(
                size_of::<u8>() * self.capacity.get(),
                Self::ALIGN,
            )
            .is_ok());

            // Deallocate all memory that the bucket allocated
            dealloc(
                items,
                // Safety: ALIGN is a non-zero power of two (checked at construction)
                //         and the size will not overflow when rounded up
                Layout::from_size_align_unchecked(size_of::<u8>() * self.capacity.get(), Self::ALIGN),
            );
        }
    }
}

unsafe impl<T: Internable> Send for Bucket<T> {}
unsafe impl<T: Internable> Sync for Bucket<T> {}
