// Unsafe blocks are used within unsafe functions for clarity on what is
// unsafe code and why it's sound
#![allow(unused_unsafe)]

use crate::{
    arenas::atomic_bucket::{AtomicBucket, AtomicBucketList},
    Capacity, LassoError, LassoErrorKind, LassoResult, MemoryLimits,
};
use alloc::format;
use core::{
    self, cmp,
    fmt::{self, Debug},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

/// An arena allocator that dynamically grows in size when needed, allocating memory in large chunks
pub(crate) struct LockfreeArena {
    /// All the internal buckets, storing all allocated and unallocated items
    buckets: AtomicBucketList,
    /// The default capacity of each bucket
    bucket_capacity: NonZeroUsize,
    memory_usage: AtomicUsize,
    pub(crate) max_memory_usage: usize,
}

impl LockfreeArena {
    /// Create a new Arena with the default bucket size of 4096 bytes
    pub fn new(capacity: NonZeroUsize, max_memory_usage: usize) -> LassoResult<Self> {
        Ok(Self {
            // Allocate one bucket
            buckets: AtomicBucketList::new(capacity)?,
            bucket_capacity: capacity,
            // The current capacity is whatever size the bucket we just allocated is
            memory_usage: AtomicUsize::new(capacity.get()),
            max_memory_usage,
        })
    }

    pub(crate) fn memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Doesn't actually allocate anything, but increments `self.memory_usage` and returns `None` if
    /// the attempted amount surpasses `max_memory_usage`
    // TODO: Make this return a `Result`
    fn allocate_memory(&mut self, requested_mem: usize) -> LassoResult<()> {
        if self.memory_usage.load(Ordering::Relaxed) + requested_mem > self.max_memory_usage {
            Err(LassoError::new(LassoErrorKind::MemoryLimitReached))
        } else {
            self.memory_usage
                .fetch_add(requested_mem, Ordering::Relaxed);

            Ok(())
        }
    }

    /// Store a slice in the Arena, returning `None` if memory is exhausted
    ///
    /// # Safety
    ///
    /// The reference passed back must be dropped before the arena that created it is
    ///
    pub unsafe fn store_str(&mut self, string: &str) -> LassoResult<&'static str> {
        let slice = string.as_bytes();
        // Ensure the length is at least 1, mainly for empty strings
        // This theoretically wastes a single byte, but it shouldn't matter since
        // the interner should ensure that only one empty string is ever interned
        let len = cmp::max(slice.len(), 1);

        // TODO: Gain exclusive access to the bucket
        if let Some(bucket) = self
            .buckets
            .iter()
            .find(|&bucket| AtomicBucket::free_elements(bucket) >= len)
        {
            // Safety: The bucket found has enough room for the slice
            unsafe { return Ok(AtomicBucket::push_slice(bucket, slice)) };
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
            let mut bucket =
                AtomicBucket::with_capacity(unsafe { NonZeroUsize::new_unchecked(len) })?;

            // Safety: The new bucket will have exactly enough room for the string and we have
            //         exclusive access to the bucket since we just created it
            let allocated_string = unsafe { AtomicBucket::push_slice(bucket, slice) };
            self.buckets.push_front(bucket);

            Ok(allocated_string)
        } else {
            let memory_usage = self.memory_usage.load(Ordering::Relaxed);

            // If trying to use the doubled capacity will surpass our memory limit, just allocate as much as we can
            if memory_usage + next_capacity > self.max_memory_usage {
                let remaining_memory = self.max_memory_usage.saturating_sub(memory_usage);

                // Check that we haven't exhausted our memory limit
                self.allocate_memory(remaining_memory)?;

                // Set the capacity to twice of what it currently is to allow for fewer allocations as more strings are interned
                let mut bucket = AtomicBucket::with_capacity(
                    NonZeroUsize::new(remaining_memory)
                        .ok_or_else(|| LassoError::new(LassoErrorKind::MemoryLimitReached))?,
                )?;

                // Safety: The new bucket will have exactly enough room for the string and we have
                //         exclusive access to the bucket since we just created it
                let allocated_string = unsafe { AtomicBucket::push_slice(bucket, slice) };
                // TODO: Push the bucket to the back or something so that we can get it somewhat out
                //       of the search path, reduce the `n` in the `O(n)` list traversal
                self.buckets.push_front(bucket);

                Ok(allocated_string)

            // Otherwise just allocate a normal doubled bucket
            // TODO: Gain exclusive access to the bucket
            } else {
                // Check that we haven't exhausted our memory limit
                self.allocate_memory(next_capacity)?;

                // Set the capacity to twice of what it currently is to allow for fewer allocations as more strings are interned
                // Safety: capacity will always be >= 1
                self.bucket_capacity = unsafe { NonZeroUsize::new_unchecked(next_capacity) };
                let mut bucket = AtomicBucket::with_capacity(self.bucket_capacity)?;

                // Safety: The new bucket will have enough room for the string
                let allocated_string = unsafe { AtomicBucket::push_slice(bucket, slice) };
                self.buckets.push_front(bucket);

                Ok(allocated_string)
            }
        }
    }
}

impl Default for LockfreeArena {
    fn default() -> Self {
        Self::new(
            Capacity::default().bytes,
            MemoryLimits::default().max_memory_usage,
        )
        .expect("failed to create default arena")
    }
}

impl Debug for LockfreeArena {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string() {
        let mut arena = LockfreeArena::default();

        unsafe {
            let idx = arena.store_str("test");

            assert_eq!(idx, Ok("test"));
        }
    }

    #[test]
    fn empty_str() {
        let mut arena = LockfreeArena::default();

        unsafe {
            let zst = arena.store_str("");
            let zst1 = arena.store_str("");
            let zst2 = arena.store_str("");

            assert_eq!(zst, Ok(""));
            assert_eq!(zst1, Ok(""));
            assert_eq!(zst2, Ok(""));
        }
    }

    #[test]
    fn exponential_allocations() {
        let mut arena = LockfreeArena::default();

        let mut len = 4096;
        for _ in 0..10 {
            let large_string = "a".repeat(len);
            let arena_string = unsafe { arena.store_str(&large_string) };
            assert_eq!(arena_string, Ok(large_string.as_str()));

            len *= 2;
        }
    }

    #[test]
    fn memory_exhausted() {
        let mut arena = LockfreeArena::new(NonZeroUsize::new(10).unwrap(), 10).unwrap();

        unsafe {
            assert!(arena.store_str("0123456789").is_ok());
            // A ZST takes up a single byte
            let err = arena.store_str("").unwrap_err();
            assert!(err.kind().is_memory_limit());
            let err = arena.store_str("dfgsagdfgsdf").unwrap_err();
            assert!(err.kind().is_memory_limit());
        }
    }

    #[test]
    fn allocate_too_much() {
        let mut arena = LockfreeArena::new(NonZeroUsize::new(1).unwrap(), 10).unwrap();

        unsafe {
            let err = arena.store_str("abcdefghijklmnopqrstuvwxyz").unwrap_err();
            assert!(err.kind().is_memory_limit());
        }
    }

    #[test]
    fn allocate_more_than_double() {
        let mut arena = LockfreeArena::new(NonZeroUsize::new(1).unwrap(), 1000).unwrap();

        unsafe {
            assert!(arena.store_str("abcdefghijklmnopqrstuvwxyz").is_ok());
        }
    }
}
