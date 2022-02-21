use crate::{LassoError, LassoErrorKind, LassoResult};
use alloc::alloc::{alloc, dealloc, Layout};
use core::{
    mem::{align_of, size_of},
    num::NonZeroUsize,
    ptr::{self, addr_of_mut, NonNull},
    slice,
    sync::atomic::{AtomicPtr, Ordering},
};

pub struct AtomicBucketList {
    /// The first bucket in the list, will be null if the list currently
    /// has no buckets
    head: AtomicPtr<AtomicBucket>,
}

impl AtomicBucketList {
    /// Create a new bucket list
    pub fn new(first_bucket_capacity: NonZeroUsize) -> LassoResult<Self> {
        let mut bucket = AtomicBucket::with_capacity(first_bucket_capacity)?;

        Ok(Self {
            head: AtomicPtr::new(bucket.as_ptr()),
        })
    }

    pub fn iter(&self) -> AtomicBucketIter<'_> {
        AtomicBucketIter {
            current: &self.head,
        }
    }

    /// Get the number of buckets within the current list
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Returns `true` if there's no buckets within the current list
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_front(&self, bucket: NonNull<AtomicBucket>) {
        let bucket_ptr = bucket.as_ptr();
        let mut head_ptr = self.head.load(Ordering::Acquire);

        loop {
            // The new bucket will become the head of the list, so we rewrite its next
            // pointer to point to the next bucket (the previous head of the list)
            unsafe {
                addr_of_mut!((*bucket_ptr).next).write(AtomicPtr::new(head_ptr));
            }

            // Replace the old head pointer with the pointer to our new bucket
            let exchange = self.head.compare_exchange_weak(
                head_ptr,
                bucket_ptr,
                // TODO: I think it's correct, but should both failure and success orderings be acquire?
                Ordering::Acquire,
                Ordering::Acquire,
            );

            // The exchange failed, set the head pointer to the new head node
            if let Err(new_head) = exchange {
                head_ptr = new_head;

            // Otherwise we succeeded swapping the pointers and are done
            } else {
                break;
            }
        }
    }
}

impl Drop for AtomicBucketList {
    fn drop(&mut self) {
        // Safety: We should have exclusive access to all buckets
        unsafe {
            let mut head_ptr = self.head.load(Ordering::Acquire);

            while !head_ptr.is_null() {
                // Grab the next pointer
                head_ptr = (*head_ptr).next.load(Ordering::Acquire);

                // Get the layout of the bucket to be deallocated
                let capacity = (*head_ptr).capacity;
                let layout = AtomicBucket::layout(capacity)
                    .expect("buckets with invalid capacities can't be constructed");

                // Deallocate all memory that the bucket allocated
                dealloc(head_ptr.cast(), layout);
            }
        }
    }
}

pub struct AtomicBucketIter<'a> {
    current: &'a AtomicPtr<AtomicBucket>,
}

impl Iterator for AtomicBucketIter<'_> {
    type Item = NonNull<AtomicBucket>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.load(Ordering::Acquire);

        NonNull::new(current).map(|current| {
            // Safety: `current` is valid and not null
            self.current = unsafe { &(*current.as_ptr()).next };
            current
        })
    }
}

#[repr(C)]
pub(super) struct AtomicBucket {
    /// The next bucket in the list, will be null if this is the last bucket
    next: AtomicPtr<Self>,
    /// The start of uninitialized memory within `items`
    index: usize,
    /// The total number of bytes allocated within the bucket
    capacity: NonZeroUsize,
    /// The inline allocated data of this bucket
    _data: [u8; 0],
}

impl AtomicBucket {
    /// Allocates a bucket with space for `capacity` items
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> LassoResult<NonNull<Self>> {
        // Create the bucket's layout
        let layout = Self::layout(capacity)?;

        // Allocate memory for the bucket
        let ptr = unsafe {
            NonNull::new(alloc(layout))
                .ok_or_else(|| LassoError::new(LassoErrorKind::FailedAllocation))?
                .cast::<Self>()
        };

        // Write to the fields of the bucket
        unsafe {
            let ptr = ptr.as_ptr();

            addr_of_mut!((*ptr).next).write(AtomicPtr::new(ptr::null_mut()));
            addr_of_mut!((*ptr).index).write(0);
            addr_of_mut!((*ptr).capacity).write(capacity);

            // We leave the allocated data uninitialized
        }

        Ok(ptr)
    }

    /// Create the layout for a bucket
    ///
    /// # Safety
    ///
    /// `capacity` must be a power of two that won't overflow when rounded up
    ///
    fn layout(capacity: NonZeroUsize) -> LassoResult<Layout> {
        let next = Layout::new::<AtomicPtr<Self>>();
        let index = Layout::new::<usize>();
        let cap = Layout::new::<NonZeroUsize>();

        // Safety: Align will always be a non-zero power of two and the
        //         size will not overflow when rounded up
        debug_assert!(
            Layout::from_size_align(size_of::<u8>() * capacity.get(), align_of::<u8>()).is_ok()
        );
        let data = unsafe {
            Layout::from_size_align_unchecked(size_of::<u8>() * capacity.get(), align_of::<u8>())
        };

        next.extend(index)
            .and_then(|(layout, _)| layout.extend(cap))
            .and_then(|(layout, _)| layout.extend(data))
            .map(|(layout, _)| layout.pad_to_align())
            .map_err(|_| LassoError::new(LassoErrorKind::FailedAllocation))
    }

    /// Get the number of available slots for the current bucket
    ///
    /// # Safety: `this` must be a valid pointer
    ///
    pub(crate) unsafe fn free_elements(this: NonNull<Self>) -> usize {
        let index = (*this.as_ptr()).index;
        let capacity = (*this.as_ptr()).capacity.get();

        capacity - index
    }

    /// Returns whether the current bucket is full
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
    pub(crate) unsafe fn push_slice(this: NonNull<Self>, slice: &[u8]) -> &'static str {
        let index = (*this.as_ptr()).index;

        if cfg!(debug_assertions) {
            let capacity = (*this.as_ptr()).capacity.get();

            debug_assert_ne!(index, capacity);
            debug_assert!(slice.len() <= capacity - index);
        }

        // Get a pointer to the start of the free data
        let ptr = addr_of_mut!((*this.as_ptr())._data).cast::<u8>().add(index);

        // Make the slice that we'll fill with the string's data
        let target = slice::from_raw_parts_mut(ptr, slice.len());
        // Copy the data from the source string into the bucket's buffer
        target.copy_from_slice(slice);

        // Increment the index so that the string we just added isn't overwritten
        addr_of_mut!((*this.as_ptr()).index).write(index + slice.len());

        // Create a string from that slice
        // Safety: The source string was valid utf8, so the created buffer will be as well
        core::str::from_utf8_unchecked(target)
    }
}

unsafe impl Send for AtomicBucket {}
unsafe impl Sync for AtomicBucket {}
