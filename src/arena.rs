// Unsafe blocks are used within unsafe functions for clarity on what is
// unsafe code and why it's sound
#![allow(unused_unsafe)]

compile! {
    if #[feature = "no-std"] {
        use alloc::{
            alloc::{alloc, dealloc, Layout},
            format,
            vec::Vec,
            vec,
        };
    } else {
        use std::alloc::{alloc, dealloc, Layout};
    }
}
use crate::{Capacity, MemoryLimits};
use core::{cmp, fmt, mem, num::NonZeroUsize, ptr::NonNull, slice};

/// An arena allocator that dynamically grows in size when needed, allocating memory in large chunks
pub(crate) struct Arena {
    /// All the internal buckets, storing all allocated and unallocated items
    buckets: Vec<Bucket>,
    /// The default capacity of each bucket
    bucket_capacity: NonZeroUsize,
    memory_usage: usize,
    pub(crate) max_memory_usage: usize,
}

impl Arena {
    /// Create a new Arena with the default bucket size of 4096 bytes
    #[inline]
    pub fn new(capacity: NonZeroUsize, max_memory_usage: usize) -> Self {
        Self {
            // Allocate one bucket
            buckets: vec![Bucket::with_capacity(capacity)],
            bucket_capacity: capacity,
            // The current capacity is whatever size the bucket we just allocated is
            memory_usage: capacity.get(),
            max_memory_usage,
        }
    }

    /// Doesn't actually allocate anything, but increments `self.memory_usage` and returns `None` if
    /// the attempted amount surpasses `max_memory_usage`
    // TODO: Make this return a `Result`
    fn allocate_memory(&mut self, requested_mem: usize) -> Option<()> {
        if self.memory_usage + requested_mem > self.max_memory_usage {
            None
        } else {
            self.memory_usage += requested_mem;

            Some(())
        }
    }

    /// Store a slice in the Arena, returning `None` if memory is exhausted
    ///
    /// # Safety
    ///
    /// The reference passed back must be dropped before the arena that created it is
    ///
    #[inline]
    pub unsafe fn store_str(&mut self, string: &str) -> Option<&'static str> {
        let slice = string.as_bytes();
        // Ensure the length is at least 1, mainly for empty strings
        // This theoretically wastes a single byte, but it shouldn't matter since
        // the interner should ensure that only one empty string is ever interned
        let len = cmp::max(slice.len(), 1);

        if let Some(bucket) = self
            .buckets
            .last_mut()
            .filter(|bucket| bucket.free_elements() >= len)
        {
            // Safety: The bucket found has enough room for the slice
            return unsafe { Some(bucket.push_slice(slice)) };
        }

        // SPEED: This portion of the code could be pulled into a cold path

        let next_capacity = self.bucket_capacity.get() * 2;

        // If the current string's length is greater than the doubled current capacity, allocate a bucket exactly the
        // size of the large string and push it back in the buckets vector. This ensures that obscenely large strings will
        // not permanently affect the resource consumption of the interner
        if len > next_capacity {
            // Check that we haven't exhausted our memory limit
            self.allocate_memory(len)?;

            // Safety: len will always be >= 1
            let mut bucket = Bucket::with_capacity(unsafe { NonZeroUsize::new_unchecked(len) });

            // Safety: The new bucket will have exactly enough room for the string
            let allocated_string = unsafe { bucket.push_slice(slice) };
            self.buckets.insert(self.buckets.len() - 2, bucket);

            Some(allocated_string)

        // If trying to use the doubled capacity will surpass our memory limit, just allocate as much as we can
        } else if self.memory_usage + next_capacity > self.max_memory_usage {
            let remaining_memory = self.max_memory_usage.saturating_sub(self.memory_usage);
            // Check that we haven't exhausted our memory limit
            self.allocate_memory(remaining_memory)?;

            // Set the capacity to twice of what it currently is to allow for fewer allocations as more strings are interned
            let mut bucket = Bucket::with_capacity(NonZeroUsize::new(remaining_memory)?);

            // Safety: The new bucket will have enough room for the string
            let allocated_string = unsafe { bucket.push_slice(slice) };
            self.buckets.push(bucket);

            Some(allocated_string)

        // Otherwise just allocate a normal doubled bucket
        } else {
            // Check that we haven't exhausted our memory limit
            self.allocate_memory(next_capacity)?;

            // Set the capacity to twice of what it currently is to allow for fewer allocations as more strings are interned
            // Safety: capacity will always be >= 1
            self.bucket_capacity = unsafe { NonZeroUsize::new_unchecked(next_capacity) };
            let mut bucket = Bucket::with_capacity(self.bucket_capacity);

            // Safety: The new bucket will have enough room for the string
            let allocated_string = unsafe { bucket.push_slice(slice) };
            self.buckets.push(bucket);

            Some(allocated_string)
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(
            Capacity::default().bytes,
            MemoryLimits::default().max_memory_usage,
        )
    }
}

impl fmt::Debug for Arena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field(
                "buckets",
                &format!(
                    "... {} bucket{}",
                    self.buckets.len(),
                    if self.buckets.len() == 1 { "" } else { "s" },
                ),
            )
            .field("bucket_capacity", &self.bucket_capacity)
            .field("memory_usage", &self.memory_usage)
            .field("max_memory_usage", &self.max_memory_usage)
            .finish()
    }
}

/// A bucket to hold a number of stored items
struct Bucket {
    /// The start of uninitialized memory within `items`
    index: usize,
    /// A pointer to the start of the data
    items: NonNull<u8>,
    /// The total number of Ts that can be stored
    capacity: NonZeroUsize,
}

impl Drop for Bucket {
    fn drop(&mut self) {
        // Safety: We have exclusive access to the pointers since the contract of
        //         `store_str` should be withheld
        unsafe {
            let items = self.items.as_ptr();

            debug_assert!(Layout::from_size_align(
                mem::size_of::<u8>() * self.capacity.get(),
                mem::align_of::<u8>(),
            )
            .is_ok());

            // Deallocate all memory that the bucket allocated
            dealloc(
                items as *mut u8,
                // Safety: Align will always be a non-zero power of two and the
                //         size will not overflow when rounded up
                Layout::from_size_align_unchecked(
                    mem::size_of::<u8>() * self.capacity.get(),
                    mem::align_of::<u8>(),
                ),
            );
        }
    }
}

impl Bucket {
    /// Allocates a bucket with space for `capacity` items
    #[inline]
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> Self {
        unsafe {
            debug_assert!(Layout::from_size_align(
                mem::size_of::<u8>() * capacity.get(),
                mem::align_of::<u8>(),
            )
            .is_ok());

            // Safety: Align will always be a non-zero power of two and the
            //         size will not overflow when rounded up
            let layout = Layout::from_size_align_unchecked(
                mem::size_of::<u8>() * capacity.get(),
                mem::align_of::<u8>(),
            );

            // Allocate the bucket's memory
            let items = NonNull::new(alloc(layout))
                // TODO: When `Result`s are piped through return this as a unique error
                .expect("Failed to allocate a new bucket, process out of memory")
                .cast();

            Self {
                index: 0,
                capacity,
                items,
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
    /// the caller promises to forget the reference before the arena is dropped.
    /// Additionally, `slice` must be valid UTF-8 and should come from an `&str`
    ///
    #[inline]
    pub(crate) unsafe fn push_slice(&mut self, slice: &[u8]) -> &'static str {
        debug_assert!(!self.is_full());
        debug_assert!(slice.len() <= self.capacity.get() - self.index);

        // Get a pointer to the start of free bytes
        let ptr = self.items.as_ptr().add(self.index);

        // Make the slice that we'll fill with the string's data
        let target = slice::from_raw_parts_mut(ptr, slice.len());
        // Copy the data from the source string into the bucket's buffer
        target.copy_from_slice(slice);
        // Increment the index so that the string we just made isn't overwritten
        self.index += slice.len();

        // Create a string from that slice
        // Safety: The source string was valid utf8, so the created buffer will be as well
        core::str::from_utf8_unchecked(target)
    }
}

unsafe impl Send for Bucket {}
unsafe impl Sync for Bucket {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string() {
        let mut arena = Arena::default();

        unsafe {
            let idx = arena.store_str("test");

            assert_eq!(idx, Some("test"));
        }
    }

    #[test]
    fn empty_str() {
        let mut arena = Arena::default();

        unsafe {
            let zst = arena.store_str("");
            let zst1 = arena.store_str("");
            let zst2 = arena.store_str("");

            assert_eq!(zst, Some(""));
            assert_eq!(zst1, Some(""));
            assert_eq!(zst2, Some(""));
        }
    }

    #[test]
    fn exponential_allocations() {
        let mut arena = Arena::default();

        let mut len = 4096;
        for _ in 0..10 {
            let large_string = "a".repeat(len);
            let arena_string = unsafe { arena.store_str(&large_string) };
            assert_eq!(arena_string, Some(large_string.as_str()));

            len *= 2;
        }
    }

    #[test]
    fn memory_exhausted() {
        let mut arena = Arena::new(NonZeroUsize::new(10).unwrap(), 10);

        unsafe {
            assert!(arena.store_str("0123456789").is_some());
            // A ZST takes up a single byte
            assert!(arena.store_str("").is_none());
            assert!(arena.store_str("dfgsagdfgsdf").is_none());
        }
    }
}
