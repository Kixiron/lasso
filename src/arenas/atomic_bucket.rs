use crate::{
    rodeo::{Internable, InternableRef},
    LassoError, LassoErrorKind, LassoResult,
};
use alloc::alloc::{alloc, dealloc, Layout};
use core::{
    hint,
    marker::PhantomData,
    mem::size_of,
    num::NonZeroUsize,
    ptr::{self, addr_of_mut, NonNull},
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

pub(super) struct AtomicBucketList<T: Internable> {
    /// The first bucket in the list, will be null if the list currently
    /// has no buckets
    head: AtomicPtr<AtomicBucket<T>>,
}

impl<T: Internable> AtomicBucketList<T> {
    /// Create a new bucket list
    pub fn new(first_bucket_capacity: NonZeroUsize) -> LassoResult<Self> {
        let bucket = AtomicBucket::<T>::with_capacity(first_bucket_capacity)?;

        Ok(Self {
            head: AtomicPtr::new(bucket.as_ptr()),
        })
    }

    pub fn iter(&self) -> AtomicBucketIter<'_, T> {
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

    pub fn push_front(&self, bucket: BucketRef<T>) {
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
}

impl<T: Internable> Drop for AtomicBucketList<T> {
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
                let layout = AtomicBucket::<T>::layout(capacity)
                    .expect("buckets with invalid capacities can't be constructed");

                // Deallocate all memory that the bucket allocated
                dealloc(current_ptr.cast(), layout);
            }
        }
    }
}

pub(super) struct AtomicBucketIter<'a, T: Internable> {
    current: &'a AtomicPtr<AtomicBucket<T>>,
}

impl<'a, T: Internable> Iterator for AtomicBucketIter<'a, T> {
    type Item = BucketRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.load(Ordering::Acquire);

        NonNull::new(current).map(|current| {
            // Safety: `current` is valid and not null
            self.current = unsafe { &(*current.as_ptr()).next };

            // Safety: `current` points to a valid bucket
            unsafe { BucketRef::new(current) }
        })
    }
}

/// A unique reference to an atomic bucket
#[repr(transparent)]
pub(super) struct UniqueBucketRef<T: Internable> {
    bucket: BucketRef<T>,
}

impl<T: Internable> UniqueBucketRef<T> {
    /// Create a new unique bucket ref
    ///
    /// # Safety
    ///
    /// The pointer must have exclusive, mutable and unique access to the pointed-to
    /// bucket
    #[inline]
    const unsafe fn new(bucket: NonNull<AtomicBucket<T>>) -> Self {
        Self {
            bucket: unsafe { BucketRef::new(bucket) },
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut AtomicBucket<T> {
        self.bucket.as_ptr()
    }

    /// Get the current bucket's length
    #[inline]
    pub fn len(&self) -> usize {
        unsafe { *(*addr_of_mut!((*self.as_ptr()).len)).get_mut() }
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
        unsafe { *(*addr_of_mut!((*self.as_ptr()).len)).get_mut() = new_length };
    }

    /// Push a slice of bytes to the current bucket
    ///
    /// # Safety
    ///
    /// The returned `&'static T::Ref` (and all copies of it) must be dropped
    /// before the current bucket is, as this bucket contains the backing
    /// memory for the data.
    /// Additionally, the underlying [`AtomicBucket`] must have enough room
    /// to store the entire value (including alignment padding).
    ///
    pub unsafe fn push_slice(&mut self, value: &T::Ref) -> &'static T::Ref {
        let len = self.len();
        let slice = value.as_bytes();
        let count = value.len();

        // Align the index to the required alignment for T::Ref
        let align = T::Ref::ALIGNMENT;
        let aligned_len = (len + align - 1) & !(align - 1);

        if cfg!(debug_assertions) {
            let capacity = self.capacity().get();

            debug_assert_ne!(aligned_len, capacity);
            debug_assert!(aligned_len + slice.len() <= capacity);
        }

        // Get a pointer to the aligned start of the free data
        let ptr = unsafe {
            addr_of_mut!((*self.as_ptr())._data)
                .cast::<u8>()
                .add(aligned_len)
        };

        // Copy the data from the source into the bucket's buffer
        unsafe { ptr.copy_from_nonoverlapping(slice.as_ptr(), slice.len()) };

        // Increment the index so that the data we just added isn't overwritten
        // Safety: All bytes are initialized and the length is <= capacity
        unsafe { self.set_len(aligned_len + slice.len()) };

        // Create a reference from the allocated data
        // Safety: The source data was valid, so the created buffer will be as well.
        // The pointer is properly aligned because we aligned the index above.
        unsafe { T::Ref::from_raw_parts(ptr, count) }
    }

    #[inline]
    pub(crate) fn into_ref(self) -> BucketRef<T> {
        self.bucket
    }
}

/// A reference to an [`AtomicBucket`]
#[repr(transparent)]
pub(super) struct BucketRef<T: Internable> {
    bucket: NonNull<AtomicBucket<T>>,
}

impl<T: Internable> BucketRef<T> {
    /// Create a new [`BucketRef`]
    ///
    /// # Safety
    ///
    /// `bucket` must be a valid pointer to an [`AtomicBucket`]
    const unsafe fn new(bucket: NonNull<AtomicBucket<T>>) -> Self {
        Self { bucket }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut AtomicBucket<T> {
        self.bucket.as_ptr()
    }

    /// Get the bucket's length
    #[inline]
    fn length(&self) -> &AtomicUsize {
        // Safety: `bucket` is a valid pointer to a bucket
        unsafe { &(*self.as_ptr()).len }
    }

    /// Get the bucket's capacity
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        // Safety: `bucket` is a valid pointer to a bucket
        unsafe { (*self.as_ptr()).capacity }
    }

    /// Get a slice pointer to the specified data range
    #[inline]
    pub unsafe fn slice_mut(&self, start: usize) -> *mut u8 {
        unsafe { addr_of_mut!((*self.as_ptr())._data).cast::<u8>().add(start) }
    }

    /// Try to atomically reserve space for `additional` bytes with the given alignment.
    /// Returns the aligned start position on success.
    pub fn try_inc_length(&self, additional: usize) -> Result<usize, ()> {
        debug_assert_ne!(additional, 0);

        let length = self.length();
        let capacity = self.capacity().get();
        let align = T::Ref::ALIGNMENT;

        // TODO: Add backoff to this loop so we don't thrash it
        let mut len = length.load(Ordering::Acquire);
        for _ in 0..100 {
            // Compute the aligned start position
            let aligned_start = (len + align - 1) & !(align - 1);
            let new_length = aligned_start + additional;

            if new_length <= capacity {
                match length.compare_exchange_weak(
                    len,
                    new_length,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        debug_assert!(aligned_start < capacity && new_length <= capacity);
                        return Ok(aligned_start);
                    }
                    Err(loaded) => {
                        hint::spin_loop();
                        len = loaded;
                    }
                }
            } else {
                break;
            }
        }

        Err(())
    }
}

#[repr(C)]
pub(super) struct AtomicBucket<T: Internable> {
    /// The next bucket in the list, will be null if this is the last bucket
    next: AtomicPtr<Self>,

    /// The start of uninitialized memory within `items`
    ///
    /// Invariant: `len` will always be less than or equal to `capacity`
    len: AtomicUsize,

    /// The total number of bytes allocated within the bucket
    capacity: NonZeroUsize,

    /// The inline allocated data of this bucket
    ///
    /// Invariant: Never touch this field manually, it contains uninitialized data up
    /// to the length of `capacity`
    _data: [u8; 0],

    /// Marker for the internable type
    _marker: PhantomData<T>,
}

impl<T: Internable> AtomicBucket<T> {
    const ALIGN: usize = T::Ref::ALIGNMENT;

    /// Allocates a bucket with space for `capacity` items
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> LassoResult<UniqueBucketRef<T>> {
        // Create the bucket's layout
        let layout = Self::layout(capacity)?;
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
            addr_of_mut!((*ptr).len).write(AtomicUsize::new(0));
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
    fn layout(capacity: NonZeroUsize) -> LassoResult<Layout> {
        let next = Layout::new::<AtomicPtr<Self>>();
        let len = Layout::new::<usize>();
        let cap = Layout::new::<NonZeroUsize>();

        // Safety: ALIGN is a non-zero power of two (checked at construction) and the
        //         size will not overflow when rounded up
        debug_assert!(Layout::from_size_align(size_of::<u8>() * capacity.get(), Self::ALIGN).is_ok());
        let data =
            unsafe { Layout::from_size_align_unchecked(size_of::<u8>() * capacity.get(), Self::ALIGN) };

        next.extend(len)
            .and_then(|(layout, _)| layout.extend(cap))
            .and_then(|(layout, _)| layout.extend(data))
            .map(|(layout, _)| layout.pad_to_align())
            .map_err(|_| LassoError::new(LassoErrorKind::FailedAllocation))
    }
}

unsafe impl<T: Internable> Send for AtomicBucket<T> {}
unsafe impl<T: Internable> Sync for AtomicBucket<T> {}
