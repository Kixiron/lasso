use crate::{arenas::LockfreeArena, leaky::ThinStrPtr, LassoResult};
use core::{
    fmt::{self, Debug},
    num::NonZeroUsize,
};

pub trait ConcurrentArena: Sized + Debug {
    type Stored;

    fn new(capacity: NonZeroUsize, max_memory_usage: usize) -> LassoResult<Self>;

    fn current_memory_usage(&self) -> usize;

    fn set_max_memory_usage(&self, max_memory_usage: usize);

    fn get_max_memory_usage(&self) -> usize;

    unsafe fn store_str(&self, string: &str) -> LassoResult<Self::Stored>;

    unsafe fn clear(&self);
}

#[repr(transparent)]
pub struct ThinStrArena {
    arena: LockfreeArena,
}

impl ConcurrentArena for ThinStrArena {
    type Stored = ThinStrPtr;

    #[inline]
    fn new(capacity: NonZeroUsize, max_memory_usage: usize) -> LassoResult<Self> {
        Ok(Self {
            arena: LockfreeArena::new(capacity, max_memory_usage)?,
        })
    }

    #[inline]
    fn current_memory_usage(&self) -> usize {
        self.arena.current_memory_usage()
    }

    #[inline]
    fn set_max_memory_usage(&self, max_memory_usage: usize) {
        self.arena.set_max_memory_usage(max_memory_usage)
    }

    #[inline]
    fn get_max_memory_usage(&self) -> usize {
        self.arena.get_max_memory_usage()
    }

    #[inline]
    unsafe fn store_str(&self, string: &str) -> LassoResult<Self::Stored> {
        unsafe { self.arena.store_prefixed_str(string).map(ThinStrPtr) }
    }

    #[inline]
    unsafe fn clear(&self) {
        unsafe { self.arena.clear() };
    }
}

impl Debug for ThinStrArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.arena.fmt(f)
    }
}

#[repr(transparent)]
pub struct InlineStrArena {
    arena: LockfreeArena,
}

impl ConcurrentArena for InlineStrArena {
    type Stored = &'static str;

    #[inline]
    fn new(capacity: NonZeroUsize, max_memory_usage: usize) -> LassoResult<Self> {
        Ok(Self {
            arena: LockfreeArena::new(capacity, max_memory_usage)?,
        })
    }

    #[inline]
    fn current_memory_usage(&self) -> usize {
        self.arena.current_memory_usage()
    }

    #[inline]
    fn set_max_memory_usage(&self, max_memory_usage: usize) {
        self.arena.set_max_memory_usage(max_memory_usage)
    }

    #[inline]
    fn get_max_memory_usage(&self) -> usize {
        self.arena.get_max_memory_usage()
    }

    #[inline]
    unsafe fn store_str(&self, string: &str) -> LassoResult<Self::Stored> {
        unsafe { self.arena.store_str(string) }
    }

    #[inline]
    unsafe fn clear(&self) {
        unsafe { self.arena.clear() };
    }
}

impl Debug for InlineStrArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.arena.fmt(f)
    }
}
