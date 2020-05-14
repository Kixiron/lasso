#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "nightly", feature(hash_raw_entry))]
#![warn(clippy::missing_inline_in_public_items)]
#![deny(
    missing_docs,
    missing_debug_implementations,
    clippy::missing_safety_doc
)]

//! [![CI][1]][0]
//! [![Security Audit][2]][0]
//! [![Coverage][3]][4]
//! [![LoC][5]][0]
//! [![Docs.rs][6]][7]
//! [![Crates.io][8]][9]
//!
//! A multithreaded and single threaded string interner that allows strings to be cached with a minimal memory footprint,
//! associating them with a unique [key] that can be used to retrieve them at any time. A [`Rodeo`] allows `O(1)`
//! internment and resolution and can be turned into a [`RodeoReader`] to allow for contention-free resolutions
//! with both key to str and str to key operations. It can also be turned into a [`RodeoResolver`] with only
//! key to str operations for the lowest possible memory usage.
//!
//! ## Which interner do I use?
//!
//! For single-threaded workloads [`Rodeo`] is encouraged, while multi-threaded applications should use [`ThreadedRodeo`].
//! Both of these are the only way to intern strings, but most applications will hit a stage where they are done interning
//! strings, and at that point is where the choice between [`RodeoReader`] and [`RodeoResolver`]. If the user needs to get
//! keys for strings still, then they must use the [`RodeoReader`] (although they can still transfer into a  [`RodeoResolver`])
//! at this point. For users who just need key to string resolution, the [`RodeoResolver`] gives contention-free access at the
//! minimum possible memory usage. Note that to gain access to [`ThreadedRodeo`] the `multi-threaded` feature is required.
//!
//! | Interner          | Thread-safe | Intern String | str to key | key to str | Contention Free | Memory Usage |
//! | ----------------- | :---------: | :-----------: | :--------: | :--------: | :-------------: | :----------: |
//! | [`Rodeo`]         |      ❌      |       ✅       |     ✅      |     ✅      |       N/A       |    Medium    |
//! | [`ThreadedRodeo`] |      ✅      |       ✅       |     ✅      |     ✅      |        ❌        |     Most     |
//! | [`RodeoReader`]   |      ✅      |       ❌       |     ✅      |     ✅      |        ✅        |    Medium    |
//! | [`RodeoResolver`] |      ✅      |       ❌       |     ❌      |     ✅      |        ✅        |    Least     |
//!
//! ## Cargo Features
//!
//!
//! By default `lasso` has zero dependencies and only [`Rodeo`] is exposed. To make use of [`ThreadedRodeo`], you must enable the `multi-threaded` feature.
//!
//! * `multi-threaded` - Enables [`ThreadedRodeo`], the interner for multi-threaded tasks
//! * `parking_locks` - Uses [`parking_lot`] for the internal concurrent locks
//! * `hashbrown-table` - Uses [`hashbrown`] as the internal `HashMap`
//! * `ahasher` - Use [`ahash`]'s `RandomState` as the default hasher
//! * `no-std` - Enables `no_std` + `alloc` support for [`Rodeo`] and [`ThreadedRodeo`]
//!   * Automatically enables the following required features:
//!     * `dashmap/no_std` - `no_std` compatibility for `DashMap`
//!     * `parking_locks` - `no_std` locks
//!     * `hashbrown-table` - `no_std` `HashMap`
//!     * `ahasher` - `no_std` hashing function
//!
//! ## Example: Using Rodeo
//!
//! ```rust
//! use lasso::Rodeo;
//!
//! let mut rodeo = Rodeo::default();
//! let key = rodeo.get_or_intern("Hello, world!");
//!
//! // Easily retrieve the value of a key and find the key for values
//! assert_eq!("Hello, world!", rodeo.resolve(&key));
//! assert_eq!(Some(key), rodeo.get("Hello, world!"));
//!
//! // Interning the same string again will yield the same key
//! let key2 = rodeo.get_or_intern("Hello, world!");
//!
//! assert_eq!(key, key2);
//! ```
//!
//! ## Example: Using ThreadedRodeo
//!
//! ```rust
//! # // This is hacky to the extreme, but it prevents failure of this doc test when
//! # // run with `--no-default-features`
//! #
//! # #[cfg(not(feature = "multi-threaded"))]
//! # #[derive(Default)]
//! # struct ThreadedRodeo;
//! #
//! # #[cfg(not(feature = "multi-threaded"))]
//! # impl ThreadedRodeo {
//! #     fn get_or_intern(&self, string: &'static str) -> i32 {
//! #         match string {
//! #             "Hello, world!" => 0,
//! #             "Hello from the thread!" => 1,
//! #             _ => unreachable!("Update the docs, dude"),
//! #         }
//! #     }
//! #
//! #     fn get(&self, string: &'static str) -> Option<i32> {
//! #         match string {
//! #             "Hello, world!" => Some(0),
//! #             "Hello from the thread!" => Some(1),
//! #             _ => unreachable!("Update the docs, dude"),
//! #         }
//! #     }
//! #
//! #     fn resolve(&self, id: &i32) -> &'static str {
//! #         match *id {
//! #             0 => "Hello, world!",
//! #             1 => "Hello from the thread!",
//! #             _ => unreachable!("Update the docs, dude"),
//! #         }
//! #     }
//! # }
//! #
//! # #[cfg(feature = "multi-threaded")]
//! use lasso::ThreadedRodeo;
//! use std::{thread, sync::Arc};
//!
//! let rodeo = Arc::new(ThreadedRodeo::default());
//! let key = rodeo.get_or_intern("Hello, world!");
//!
//! // Easily retrieve the value of a key and find the key for values
//! assert_eq!("Hello, world!", rodeo.resolve(&key));
//! assert_eq!(Some(key), rodeo.get("Hello, world!"));
//!
//! // Interning the same string again will yield the same key
//! let key2 = rodeo.get_or_intern("Hello, world!");
//!
//! assert_eq!(key, key2);
//!
//! // ThreadedRodeo can be shared across threads
//! let moved = Arc::clone(&rodeo);
//! let hello = thread::spawn(move || {
//!     assert_eq!("Hello, world!", moved.resolve(&key));
//!     moved.get_or_intern("Hello from the thread!")
//! })
//! .join()
//! .unwrap();
//!
//! assert_eq!("Hello, world!", rodeo.resolve(&key));
//! assert_eq!("Hello from the thread!", rodeo.resolve(&hello));
//! ```
//!
//! ## Example: Creating a RodeoReader
//!
//! ```rust
//! use lasso::Rodeo;
//!
//! // Rodeo and ThreadedRodeo are interchangeable here
//! let mut rodeo = Rodeo::default();
//!
//! let key = rodeo.get_or_intern("Hello, world!");
//! assert_eq!("Hello, world!", rodeo.resolve(&key));
//!
//! let reader = rodeo.into_reader();
//!
//! // Reader keeps all the strings from the parent
//! assert_eq!("Hello, world!", reader.resolve(&key));
//! assert_eq!(Some(key), reader.get("Hello, world!"));
//!
//! // The Reader can now be shared across threads, no matter what kind of Rodeo created it
//! ```
//!
//! ## Example: Creating a RodeoResolver
//!
//! ```rust
//! use lasso::Rodeo;
//!
//! // Rodeo and ThreadedRodeo are interchangeable here
//! let mut rodeo = Rodeo::default();
//!
//! let key = rodeo.get_or_intern("Hello, world!");
//! assert_eq!("Hello, world!", rodeo.resolve(&key));
//!
//! let resolver = rodeo.into_resolver();
//!
//! // Resolver keeps all the strings from the parent
//! assert_eq!("Hello, world!", resolver.resolve(&key));
//!
//! // The Resolver can now be shared across threads, no matter what kind of Rodeo created it
//! ```
//!
//! [0]: https://github.com/Kixiron/lasso
//! [1]: https://github.com/Kixiron/lasso/workflows/Build/badge.svg
//! [2]: https://github.com/Kixiron/lasso/workflows/Security%20Audit/badge.svg
//! [3]: https://coveralls.io/repos/github/Kixiron/lasso/badge.svg?branch=master
//! [4]: https://coveralls.io/github/Kixiron/lasso?branch=master
//! [5]: https://tokei.rs/b1/github/Kixiron/lasso
//! [6]: https://docs.rs/lasso/badge.svg
//! [7]: https://docs.rs/lasso
//! [8]: https://img.shields.io/crates/v/lasso.svg
//! [9]: https://crates.io/crates/lasso
//! [key]: crate::Key
//! [`Rodeo`]: crate::Rodeo
//! [`ThreadedRodeo`]: crate::ThreadedRodeo
//! [`RodeoResolver`]: crate::RodeoResolver
//! [`RodeoReader`]: crate::RodeoReader
//! [`hashbrown`]: https://crates.io/crates/hashbrown
//! [`ahash`]: https://crates.io/crates/ahash
//! [`parking_lot`]: https://crates.io/crates/parking_lot

#[macro_use]
mod util;

// mod unique; // Experimental, doesn't currently work
mod key;
mod reader;
mod resolver;
mod single_threaded;

pub use key::{Key, LargeSpur, MicroSpur, MiniSpur, Spur};
pub use reader::RodeoReader;
pub use resolver::RodeoResolver;
pub use single_threaded::Rodeo;

compile! {
    if #[feature = "no-std"] {
        extern crate alloc;
    }

    if #[feature = "multi-threaded"] {
        mod multi_threaded;
        pub use multi_threaded::ThreadedRodeo;
    }
}

#[doc(hidden)]
mod hasher {
    compile! {
        if #[feature = "ahasher"] {
            pub use ahash::RandomState;
        } else {
            pub use std::collections::hash_map::RandomState;
        }

        if #[feature = "hashbrown-table"] {
            pub use hashbrown::HashMap;
        } else {
            pub use std::collections::HashMap;
        }
    }
}

#[doc(hidden)]
mod locks {
    compile! {
        if #[feature = "no-std"] {
            pub use alloc::sync::Arc;
        } else {
            pub use std::sync::Arc;
        }
    }
}

// TODO: No-alloc interner
