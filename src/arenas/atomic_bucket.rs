use crate::{LassoError, LassoErrorKind, LassoResult};
use alloc::alloc::{alloc, dealloc, Layout};
use core::{
    mem::{size_of, MaybeUninit},
    num::NonZeroUsize,
    ptr::{self, addr_of_mut, NonNull},
    slice,
    sync::atomic::{AtomicPtr, Ordering},
};

pub(super) struct AtomicBucketList {
    /// The first bucket in the list, will be null if the list currently
    /// has no buckets
    head: AtomicPtr<AtomicBucket>,
    align: NonZeroUsize,
}

impl AtomicBucketList {
    /// Create a new bucket list
    pub fn new(first_bucket_capacity: NonZeroUsize, align: NonZeroUsize) -> LassoResult<Self> {
        let bucket = AtomicBucket::with_capacity(first_bucket_capacity, align)?;

        Ok(Self {
            head: AtomicPtr::new(bucket.as_ptr()),
            align,
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
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_front(&self, bucket: UniqueBucketRef) {
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
                Ordering::AcqRel,
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

    pub(super) const fn align(&self) -> NonZeroUsize {
        self.align
    }
}

impl Drop for AtomicBucketList {
    fn drop(&mut self) {
        // Safety: We should have exclusive access to all buckets
        unsafe {
            let mut head_ptr = self.head.load(Ordering::Acquire);

            while !head_ptr.is_null() {
                // Keep ahold of the current pointer so we can operate over it
                let current_ptr = head_ptr;

                // Grab the next pointer and set it to be the next in line for
                // deallocation
                head_ptr = (*head_ptr).next.load(Ordering::Acquire);

                // Get the layout of the current bucket so we can deallocate it
                let capacity = (*current_ptr).capacity;
                let layout = AtomicBucket::layout(capacity, self.align)
                    .expect("buckets with invalid capacities can't be constructed");

                // Deallocate all memory that the bucket allocated
                dealloc(current_ptr.cast(), layout);
            }
        }
    }
}

pub(super) struct AtomicBucketIter<'a> {
    current: &'a AtomicPtr<AtomicBucket>,
}

impl<'a> Iterator for AtomicBucketIter<'a> {
    type Item = (&'a AtomicPtr<AtomicBucket>, BucketRef);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.load(Ordering::Acquire);

        NonNull::new(current).map(|current| {
            let parent = self.current;

            // Safety: `current` is valid and not null
            self.current = unsafe { &(*current.as_ptr()).next };

            // Safety: `current` points to a valid bucket
            (parent, unsafe { BucketRef::new(current) })
        })
    }
}

/// A unique reference to an atomic bucket
#[repr(transparent)]
pub(super) struct UniqueBucketRef {
    bucket: BucketRef,
}

impl UniqueBucketRef {
    /// Create a new unique bucket ref
    ///
    /// # Safety
    ///
    /// The pointer must have exclusive, mutable and unique access to the pointed-to
    /// bucket
    #[inline]
    const unsafe fn new(bucket: NonNull<AtomicBucket>) -> Self {
        Self {
            bucket: unsafe { BucketRef::new(bucket) },
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut AtomicBucket {
        self.bucket.as_ptr()
    }

    /// Get the current bucket's length
    #[inline]
    pub fn len(&self) -> usize {
        self.bucket.len()
    }

    /// Get the current bucket's capacity
    #[inline]
    pub fn capacity(&self) -> NonZeroUsize {
        self.bucket.capacity()
    }

    /// Set the bucket's length
    ///
    /// # Safety
    ///
    /// `new_length` must be less than or equal to the current capacity
    /// and all bytes up to `new_length` must be initialized and valid
    /// utf-8
    #[inline]
    pub unsafe fn set_len(&mut self, new_length: usize) {
        debug_assert!(
            new_length <= self.capacity().get(),
            "the bucket length {} should always be less than the bucket's capacity {}",
            new_length,
            self.capacity(),
        );

        // Safety: We have exclusive access to the bucket
        unsafe {
            addr_of_mut!((*self.as_ptr()).len).write(new_length);
        }
    }

    /// Push a slice of bytes to the current bucket
    ///
    /// # Safety
    ///
    /// The returned `&'static V` (and all copies of it) must be dropped
    /// before the current bucket is, as this bucket contains the backing
    /// memory for the string.
    /// Additionally, the underlying [`AtomicBucket`] must have enough room
    /// to store the entire slice and the given slice must be valid utf-8 data.
    ///
    pub unsafe fn allocate(&mut self, length: usize) -> &'static mut [MaybeUninit<u8>] {
        let current_length = self.len();

        if cfg!(debug_assertions) {
            let capacity = self.capacity().get();

            debug_assert_ne!(current_length, capacity);
            debug_assert!(length <= capacity - current_length);
        }

        unsafe {
            // Get a pointer to the start of the free data
            let ptr = addr_of_mut!((*self.as_ptr())._data)
                .cast::<MaybeUninit<u8>>()
                .add(current_length);

            // Increment the index so that the string we just added isn't overwritten
            self.set_len(current_length + length);

            slice::from_raw_parts_mut(ptr, length)
        }
    }
}

/// A reference to an [`AtomicBucket`]
#[repr(transparent)]
pub(super) struct BucketRef {
    bucket: NonNull<AtomicBucket>,
}

impl BucketRef {
    /// Create a new [`BucketRef`]
    ///
    /// # Safety
    ///
    /// `bucket` must be a valid pointer to an [`AtomicBucket`]
    const unsafe fn new(bucket: NonNull<AtomicBucket>) -> Self {
        Self { bucket }
    }

    /// Make a unique bucket out of the current bucket
    ///
    /// # Safety
    ///
    /// Must have exclusive access to the current bucket
    pub const unsafe fn into_unique(self) -> UniqueBucketRef {
        unsafe { UniqueBucketRef::new(self.bucket) }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut AtomicBucket {
        self.bucket.as_ptr()
    }

    #[inline]
    pub fn next_ptr(&self) -> &AtomicPtr<AtomicBucket> {
        // Safety: `bucket` is a valid pointer to a bucket
        unsafe { &(*self.as_ptr()).next }
    }

    /// Get the bucket's length
    #[inline]
    pub fn len(&self) -> usize {
        // Safety: `bucket` is a valid pointer to a bucket
        unsafe { (*self.as_ptr()).len }
    }

    /// Get the bucket's capacity
    #[inline]
    pub fn capacity(&self) -> NonZeroUsize {
        // Safety: `bucket` is a valid pointer to a bucket
        unsafe { (*self.as_ptr()).capacity }
    }

    /// Get the number of available slots for the current bucket
    #[inline]
    pub fn free_elements(&self) -> usize {
        let (len, capacity) = (self.len(), self.capacity().get());
        debug_assert!(
            len <= capacity,
            "the bucket length {} should always be less than the bucket's capacity {}",
            len,
            capacity,
        );

        capacity - len
    }
}

#[repr(C)]
pub(super) struct AtomicBucket {
    /// The next bucket in the list, will be null if this is the last bucket
    next: AtomicPtr<Self>,

    /// The start of uninitialized memory within `items`
    ///
    /// Invariant: `len` will always be less than or equal to `capacity`
    len: usize,

    /// The total number of bytes allocated within the bucket
    capacity: NonZeroUsize,

    /// The inline allocated data of this bucket
    ///
    /// Invariant: Never touch this field manually, it contains uninitialized data up
    /// to the length of `capacity`
    _data: [MaybeUninit<u8>; 0],
}

impl AtomicBucket {
    /// Allocates a bucket with space for `capacity` items
    pub(crate) fn with_capacity(
        capacity: NonZeroUsize,
        align: NonZeroUsize,
    ) -> LassoResult<UniqueBucketRef> {
        // Create the bucket's layout
        let layout = Self::layout(capacity, align)?;
        debug_assert_ne!(layout.size(), 0);

        // Allocate memory for the bucket
        // Safety: The given layout has a non-zero size
        let ptr = unsafe {
            NonNull::new(alloc(layout))
                .ok_or_else(|| LassoError::new(LassoErrorKind::FailedAllocation))?
                .cast::<Self>()
        };

        // Write to the fields of the bucket
        // Safety: We have exclusive access to the bucket and can write
        //         to its uninitialized fields
        unsafe {
            let ptr = ptr.as_ptr();

            addr_of_mut!((*ptr).next).write(AtomicPtr::new(ptr::null_mut()));
            addr_of_mut!((*ptr).len).write(0);
            addr_of_mut!((*ptr).capacity).write(capacity);

            // We leave the allocated data uninitialized, future writers will
            // initialize it as-needed
        }

        // Safety: We have exclusive access to the bucket
        Ok(unsafe { UniqueBucketRef::new(ptr) })
    }

    /// Create the layout for a bucket
    ///
    /// # Safety
    ///
    /// `capacity` must be a power of two that won't overflow when rounded up
    ///
    fn layout(capacity: NonZeroUsize, align: NonZeroUsize) -> LassoResult<Layout> {
        let next = Layout::new::<AtomicPtr<Self>>();
        let len = Layout::new::<usize>();
        let cap = Layout::new::<NonZeroUsize>();
        let data = Layout::from_size_align(size_of::<u8>() * capacity.get(), align.get())
            .map_err(|_| LassoError::new(LassoErrorKind::FailedAllocation))?;

        next.extend(len)
            .and_then(|(layout, _)| layout.extend(cap))
            .and_then(|(layout, _)| layout.extend(data))
            .map(|(layout, _)| layout.pad_to_align())
            .map_err(|_| LassoError::new(LassoErrorKind::FailedAllocation))
    }
}

unsafe impl Send for AtomicBucket {}
unsafe impl Sync for AtomicBucket {}
