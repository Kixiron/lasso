#![cfg_attr(feature = "no-std", no_std)]
// `.copied()` was unstable in 1.34
#![allow(clippy::map_clone)]
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
//! |-------------------|:-----------:|:-------------:|:----------:|:----------:|:---------------:|:------------:|
//! | [`Rodeo`]         |      ❌      |       ✅       |     ✅      |     ✅      |       N/A       |    Medium    |
//! | [`ThreadedRodeo`] |      ✅      |       ✅       |     ✅      |     ✅      |        ❌        |     Most     |
//! | [`RodeoReader`]   |      ✅      |       ❌       |     ✅      |     ✅      |        ✅        |    Medium    |
//! | [`RodeoResolver`] |      ✅      |       ❌       |     ❌      |     ✅      |        ✅        |    Least     |
//!
//! ## Cargo Features
//!
//! By default `lasso` has one dependency, `hashbrown`, and only [`Rodeo`] is exposed. Hashbrown is used since the [`raw_entry` api] is currently unstable in the standard library's hashmap.
//! The raw hashmap API is used for custom hashing within the hashmaps, which works to dramatically reduce memory usage
//! To make use of [`ThreadedRodeo`], you must enable the `multi-threaded` feature.
//!
//! * `multi-threaded` - Enables [`ThreadedRodeo`], the interner for multi-threaded tasks
//! * `ahasher` - Use [`ahash`]'s `RandomState` as the default hasher
//! * `no-std` - Enables `no_std` + `alloc` support for [`Rodeo`] and [`ThreadedRodeo`]
//!   * Automatically enables the following required features:
//!     * `ahasher` - `no_std` hashing function
//! * `serialize` - Implements `Serialize` and `Deserialize` for all `Spur` types
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
//! ## Benchmarks
//!
//! Benchmarks were gathered with [Criterion.rs](https://github.com/bheisler/criterion.rs)  
//! OS: Windows 10  
//! CPU: Ryzen 9 3900X at 3800Mhz  
//! RAM: 3200Mhz  
//! Rustc: Stable 1.44.1
//!
//! ### Rodeo
//!
//! #### STD RandomState
//!
//! | Method                       |   Time    |  Throughput  |
//! |:-----------------------------|:---------:|:------------:|
//! | `resolve`                    | 1.9125 μs | 13.373 GiB/s |
//! | `try_resolve`                | 1.9312 μs | 13.244 GiB/s |
//! | `resolve_unchecked`          | 1.4382 μs | 17.783 GiB/s |
//! | `get_or_intern` (empty)      | 59.379 μs | 441.06 MiB/s |
//! | `get_or_intern` (filled)     | 56.688 μs | 462.00 MiB/s |
//! | `try_get_or_intern` (empty)  | 59.149 μs | 442.78 MiB/s |
//! | `try_get_or_intern` (filled) | 57.373 μs | 456.49 MiB/s |
//! | `get` (empty)                | 38.040 μs | 688.49 MiB/s |
//! | `get` (filled)               | 56.733 μs | 461.63 MiB/s |
//!
//! #### AHash
//!
//! | Method                       |   Time    |  Throughput  |
//! |:-----------------------------|:---------:|:------------:|
//! | `resolve`                    | 1.9285 μs | 13.262 GiB/s |
//! | `try_resolve`                | 1.9080 μs | 13.405 GiB/s |
//! | `resolve_unchecked`          | 1.4183 μs | 18.033 GiB/s |
//! | `get_or_intern` (empty)      | 39.370 μs | 665.23 MiB/s |
//! | `get_or_intern` (filled)     | 35.424 μs | 739.33 MiB/s |
//! | `try_get_or_intern` (empty)  | 39.860 μs | 657.05 MiB/s |
//! | `try_get_or_intern` (filled) | 33.943 μs | 771.59 MiB/s |
//! | `get` (empty)                | 11.847 μs | 2.1589 GiB/s |
//! | `get` (filled)               | 33.799 μs | 774.86 MiB/s |
//!
//! #### FXHash
//!
//! | Method                       |   Time    |  Throughput  |
//! |:-----------------------------|:---------:|:------------:|
//! | `resolve`                    | 1.9182 μs | 13.333 GiB/s |
//! | `try_resolve`                | 1.9170 μs | 13.342 GiB/s |
//! | `resolve_unchecked`          | 1.4582 μs | 17.540 GiB/s |
//! | `get_or_intern` (empty)      | 32.451 μs | 807.05 MiB/s |
//! | `get_or_intern` (filled)     | 30.733 μs | 852.16 MiB/s |
//! | `try_get_or_intern` (empty)  | 32.788 μs | 798.77 MiB/s |
//! | `try_get_or_intern` (filled) | 30.398 μs | 861.55 MiB/s |
//! | `get` (empty)                | 13.541 μs | 1.8888 GiB/s |
//! | `get` (filled)               | 30.186 μs | 867.62 MiB/s |
//!
//! ### ThreadedRodeo
//!
//! #### STD RandomState
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:-----------------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `resolve`                    |    54.336 μs    |     482.00 MiB/s      |     364.27 μs     |      71.897 MiB/s       |
//! | `try_resolve`                |    54.582 μs    |     479.82 MiB/s      |     352.67 μs     |      74.261 MiB/s       |
//! | `get_or_intern` (empty)      |    266.03 μs    |     98.447 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    103.04 μs    |     254.17 MiB/s      |     441.42 μs     |      59.331 MiB/s       |
//! | `try_get_or_intern` (empty)  |    261.80 μs    |     100.04 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    102.61 μs    |     255.25 MiB/s      |     447.42 μs     |      58.535 MiB/s       |
//! | `get` (empty)                |    80.346 μs    |     325.96 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    92.669 μs    |     282.62 MiB/s      |     439.24 μs     |      59.626 MiB/s       |
//!
//! #### AHash
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:-----------------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `resolve`                    |    22.261 μs    |     1.1489 GiB/s      |     265.46 μs     |      98.658 MiB/s       |
//! | `try_resolve`                |    22.378 μs    |     1.1429 GiB/s      |     268.58 μs     |      97.513 MiB/s       |
//! | `get_or_intern` (empty)      |    157.86 μs    |     165.91 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    56.320 μs    |     465.02 MiB/s      |     357.13 μs     |      73.335 MiB/s       |
//! | `try_get_or_intern` (empty)  |    161.46 μs    |     162.21 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    55.874 μs    |     468.73 MiB/s      |     360.25 μs     |      72.698 MiB/s       |
//! | `get` (empty)                |    43.520 μs    |     601.79 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    53.720 μs    |     487.52 MiB/s      |     360.66 μs     |      72.616 MiB/s       |
//!
//! #### FXHash
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:-----------------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `try_resolve`                |    17.289 μs    |     1.4794 GiB/s      |     238.29 μs     |      109.91 MiB/s       |
//! | `resolve`                    |    19.833 μs    |     1.2896 GiB/s      |     237.05 μs     |      110.48 MiB/s       |
//! | `get_or_intern` (empty)      |    130.97 μs    |     199.97 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    42.630 μs    |     614.35 MiB/s      |     301.60 μs     |      86.837 MiB/s       |
//! | `try_get_or_intern` (empty)  |    129.30 μs    |     202.55 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    42.508 μs    |     616.12 MiB/s      |     337.29 μs     |      77.648 MiB/s       |
//! | `get` (empty)                |    28.001 μs    |     935.30 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    37.700 μs    |     694.68 MiB/s      |     292.15 μs     |      89.645 MiB/s       |
//!
//! ### RodeoReader
//!
//! #### STD RandomState
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput  (24 Threads) |
//! |:--------------------|:---------------:|:---------------------:|:-----------------:|:------------------------:|
//! | `resolve`           |    1.9398 μs    |     13.185 GiB/s      |     4.3153 μs     |       5.9269 GiB/s       |
//! | `try_resolve`       |    1.9315 μs    |     13.242 GiB/s      |     4.1956 μs     |       6.0959 GiB/s       |
//! | `resolve_unchecked` |    1.4416 μs    |     17.741 GiB/s      |     3.1204 μs     |       8.1964 GiB/s       |
//! | `get` (empty)       |    38.886 μs    |     673.50 MiB/s      |        N\A        |           N\A            |
//! | `get` (filled)      |    56.271 μs    |     465.42 MiB/s      |     105.12 μs     |       249.14 MiB/s       |
//!
//! #### AHash
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:--------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `resolve`           |    1.9404 μs    |     13.181 GiB/s      |     4.1881 μs     |      6.1069 GiB/s       |
//! | `try_resolve`       |    1.8932 μs    |     13.509 GiB/s      |     4.2410 μs     |      6.0306 GiB/s       |
//! | `resolve_unchecked` |    1.4128 μs    |     18.103 GiB/s      |     3.1691 μs     |      8.0703 GiB/s       |
//! | `get` (empty)       |    11.952 μs    |     2.1399 GiB/s      |        N\A        |           N\A           |
//! | `get` (filled)      |    27.093 μs    |     966.65 MiB/s      |     56.269 μs     |      465.44 MiB/s       |
//!
//! #### FXHash
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:--------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `resolve`           |    1.8987 μs    |     13.471 GiB/s      |     4.2117 μs     |      6.0727 GiB/s       |
//! | `try_resolve`       |    1.9103 μs    |     13.389 GiB/s      |     4.2254 μs     |      6.0529 GiB/s       |
//! | `resolve_unchecked` |    1.4469 μs    |     17.677 GiB/s      |     3.0923 μs     |      8.2709 GiB/s       |
//! | `get` (empty)       |    12.994 μs    |     1.9682 GiB/s      |        N\A        |           N\A           |
//! | `get` (filled)      |    29.745 μs    |     880.49 MiB/s      |     52.387 μs     |      499.93 MiB/s       |
//!
//! ### RodeoResolver
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! |:--------------------|:---------------:|:---------------------:|:-----------------:|:-----------------------:|
//! | `resolve`           |    1.9416 μs    |     13.172 GiB/s      |     3.9114 μs     |      6.5387 GiB/s       |
//! | `try_resolve`       |    1.9264 μs    |     13.277 GiB/s      |     3.9289 μs     |      6.5097 GiB/s       |
//! | `resolve_unchecked` |    1.6638 μs    |     15.372 GiB/s      |     3.1741 μs     |      8.0578 GiB/s       |
//!
//! [0]: https://github.com/Kixiron/lasso
//! [1]: https://github.com/Kixiron/lasso/workflows/CI/badge.svg
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
//! [`raw_entry` api]: https://github.com/rust-lang/rust/issues/56167

#[macro_use]
mod util;

mod arena;
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

    if #[all(feature = "multi-threaded", not(feature = "no-std"))] {
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
