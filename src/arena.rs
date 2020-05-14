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
pub struct Arena<T: Sized> {
    /// All the internal buckets, storing all allocated and unallocated items
    buckets: Vec<Bucket<T>>,
    /// The default capacity of each bucket
    capacity: NonZeroUsize,
}

impl<T: Sized> Arena<T> {
    /// Create a new Arena with the default bucket size of 1024 items
    ///
    /// Note: When used with ZSTs, the bucket size will always be 1
    ///
    #[inline]
    pub fn new() -> Self {
        let capacity = if mem::size_of::<T>() == 0 {
            // Only make buckets of size 1 for zsts
            unsafe { NonZeroUsize::new_unchecked(1) }
        } else {
            unsafe { NonZeroUsize::new_unchecked(1024) }
        };

        Self {
            // Leave space for a single bucket
            buckets: Vec::with_capacity(1),
            capacity,
        }
    }
}

impl<'a> Arena<u8> {
    /// Store a string in the Arena
    ///
    /// # Safety
    ///
    /// The caller promises to forget the reference before the arena is dropped
    ///
    #[inline]
    pub unsafe fn store_str(&mut self, string: &str) -> &'static str {
        let len = cmp::max(string.as_bytes().len(), 1);

        if let Some(bucket) = self
            .buckets
            .last_mut()
            .filter(|bucket| bucket.free_elements() >= len)
        {
            // Safety: The bucket found has enough room for the string
            return bucket.push_str(string);
        }

        // Safety: Length is >= 1
        let mut bucket =
            Bucket::with_capacity(cmp::max(self.capacity, NonZeroUsize::new_unchecked(len)));

        // Safety: The new bucket will have enough room for the string
        let ticket = bucket.push_str(string);
        self.buckets.push(bucket);

        ticket
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("buckets", &format!("... {} buckets", self.buckets.len()))
            .finish()
    }
}

/// A bucket to hold a number of stored items
struct Bucket<T: Sized> {
    /// The start of uninitialized memory within `items`
    index: usize,
    /// A pointer to the start of the data
    items: NonNull<T>,
    /// The total number of Ts that can be stored
    capacity: NonZeroUsize,
}

impl<T: Sized> Drop for Bucket<T> {
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

impl<T: Sized> Bucket<T> {
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
                capacity: capacity.clone(),
                items: NonNull::new(alloc(layout))
                    .expect("Failed to allocate a new bucket, process out of memory")
                    .cast(),
            }
        }
    }

    /// Get the number of avaliable slots for the current bucket
    #[inline]
    pub(crate) const fn free_elements(&self) -> usize {
        self.capacity.get() - self.index
    }

    /// Returns whether the current bucket is full
    #[inline]
    pub(crate) const fn is_full(&self) -> bool {
        self.index == self.capacity.get()
    }
}

impl Bucket<u8> {
    /// Push a string to the current bucket, returning a pointer to it
    ///
    /// # Safety
    ///
    /// The current bucket must have room for all bytes of the string and
    /// the caller promises to forget the reference before the arena is dropped
    ///
    #[inline]
    pub(crate) unsafe fn push_str<'a>(&mut self, string: &str) -> &'static str {
        debug_assert!(!self.is_full());

        let bytes = string.as_bytes();
        debug_assert!(bytes.len() <= self.capacity.get() - self.index);

        let ptr = self.items.as_ptr().add(self.index);
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());

        self.index += bytes.len();

        let str_ptr = slice::from_raw_parts_mut(ptr, bytes.len()) as *mut [u8] as *mut str;

        // Safety: The caller promises to forget the reference before the arena is dropped
        &*str_ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string() {
        let mut arena = Arena::new();

        let slice = unsafe { arena.store_str("test") };

        assert_eq!(slice, "test");
    }

    #[test]
    fn empty_str() {
        let mut arena = Arena::new();

        unsafe {
            let zst = arena.store_str("");
            let zst1 = arena.store_str("");
            let zst2 = arena.store_str("");

            assert_eq!(zst, "");
            assert_eq!(zst1, "");
            assert_eq!(zst2, "");
        }
    }
}
