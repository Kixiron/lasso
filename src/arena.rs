compile! {
    if #[feature = "no-std"] {
        use alloc::{
            alloc::{alloc, dealloc, Layout},
            format,
            vec::Vec,
        };
    } else {
        use std::alloc::{alloc, dealloc, Layout};
    }
}

use core::{
    cmp, fmt, mem,
    num::NonZeroUsize,
    ptr::{self, NonNull},
    slice,
};

/// An arena allocator that dynamically grows in size when needed, allocating memory in large chunks
pub struct Arena<T: Sized + Clone> {
    /// All the internal buckets, storing all allocated and unallocated items
    buckets: Vec<Bucket<T>>,
    /// The default capacity of each bucket
    capacity: NonZeroUsize,
}

impl<T: Sized + Clone> Arena<T> {
    /// Create a new Arena with the default bucket size of 4096 items
    ///
    /// Note: When used with ZSTs, the bucket size will always be 1
    ///
    #[inline]
    pub fn new() -> Self {
        let capacity = if mem::size_of::<T>() == 0 {
            // Only make buckets of size 1 for zsts
            unsafe { NonZeroUsize::new_unchecked(1) }
        } else {
            unsafe { NonZeroUsize::new_unchecked(4096) }
        };

        Self {
            // Leave space for a single bucket
            buckets: Vec::with_capacity(1),
            capacity,
        }
    }

    /// Store a slice in the Arena
    ///
    /// # Safety
    ///
    /// The caller promises to forget the reference before the arena is dropped
    ///
    #[inline]
    pub unsafe fn store_slice(&mut self, slice: &[T]) -> &'static [T] {
        let len = cmp::max(slice.len(), 1);

        if let Some(bucket) = self
            .buckets
            .last_mut()
            .filter(|bucket| bucket.free_elements() >= len)
        {
            // Safety: The bucket found has enough room for the slice
            return bucket.push_slice(slice);
        }

        // Safety: Length is >= 1
        let mut bucket =
            Bucket::with_capacity(cmp::max(self.capacity, NonZeroUsize::new_unchecked(len)));

        // Safety: The new bucket will have enough room for the slice
        let static_slice = bucket.push_slice(slice);
        self.buckets.push(bucket);

        static_slice
    }
}

impl<T: Clone> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("buckets", &format!("... {} buckets", self.buckets.len()))
            .finish()
    }
}

/// A bucket to hold a number of stored items
struct Bucket<T: Sized + Clone> {
    /// The start of uninitialized memory within `items`
    index: usize,
    /// A pointer to the start of the data
    items: NonNull<T>,
    /// The total number of Ts that can be stored
    capacity: NonZeroUsize,
}

impl<T: Sized + Clone> Drop for Bucket<T> {
    fn drop(&mut self) {
        // Safety: Only valid items are dropped, and then all memory is deallocated.
        // All pointers are valid.
        unsafe {
            let items = self.items.as_ptr();

            // Drop all initialized items
            for i in 0..self.index {
                ptr::drop_in_place(items.add(i));
            }

            // Deallocate all memory that the bucket allocated
            dealloc(
                items as *mut u8,
                Layout::from_size_align_unchecked(
                    mem::size_of::<T>() * self.capacity.get(),
                    mem::align_of::<T>(),
                ),
            );
        }
    }
}

impl<T: Sized + Clone> Bucket<T> {
    /// Allocates a bucket with space for `capacity` items
    #[inline]
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> Self {
        unsafe {
            let layout = Layout::from_size_align_unchecked(
                mem::size_of::<T>() * capacity.get(),
                mem::align_of::<T>(),
            );

            Self {
                index: 0,
                capacity,
                items: NonNull::new(alloc(layout))
                    .expect("Failed to allocate a new bucket, process out of memory")
                    .cast(),
            }
        }
    }

    /// Get the number of avaliable slots for the current bucket
    #[inline]
    pub(crate) fn free_elements(&self) -> usize {
        self.capacity.get() - self.index
    }

    /// Returns whether the current bucket is full
    #[inline]
    pub(crate) fn is_full(&self) -> bool {
        self.index == self.capacity.get()
    }

    /// Push a slice to the current bucket, returning a pointer to it
    ///
    /// # Safety
    ///
    /// The current bucket must have room for all bytes of the slice and
    /// the caller promises to forget the reference before the arena is dropped
    ///
    #[inline]
    pub(crate) unsafe fn push_slice(&mut self, slice: &[T]) -> &'static [T] {
        debug_assert!(!self.is_full());
        debug_assert!(slice.len() <= self.capacity.get() - self.index);

        let ptr = self.items.as_ptr().add(self.index);
        let target = slice::from_raw_parts_mut(ptr, slice.len());
        target.clone_from_slice(slice);
        self.index += slice.len();

        // Safety: The caller promises to forget the reference before the arena is dropped
        &*ptr::slice_from_raw_parts(ptr, slice.len())
    }
}

unsafe impl<T: Send + Clone> Send for Bucket<T> {}
unsafe impl<T: Sync + Clone> Sync for Bucket<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string() {
        let mut arena = Arena::new();

        let slice = unsafe { arena.store_slice("test".as_bytes()) };

        assert_eq!(slice, b"test");
    }

    #[test]
    fn empty_str() {
        let mut arena = Arena::new();

        unsafe {
            let zst = arena.store_slice("".as_bytes());
            let zst1 = arena.store_slice("".as_bytes());
            let zst2 = arena.store_slice("".as_bytes());

            assert_eq!(zst, b"");
            assert_eq!(zst1, b"");
            assert_eq!(zst2, b"");
        }
    }
}
