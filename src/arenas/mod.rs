#![allow(unsafe_op_in_unsafe_fn)]

mod atomic_bucket;
mod bucket;
mod lockfree;
mod single_threaded;

pub(crate) use lockfree::LockfreeArena;
pub(crate) use single_threaded::Arena;

use std::fmt::{self, Debug};

/// A wrapper type to abstract over all arena types
///
/// Used for readers & resolvers to allow them to be created from
/// any arena type without using dynamic dispatch or allocation
pub(crate) enum AnyArena {
    Arena(Arena),
    Lockfree(LockfreeArena),
}

impl Debug for AnyArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arena(arena) => arena.fmt(f),
            Self::Lockfree(arena) => arena.fmt(f),
        }
    }
}
