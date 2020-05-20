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
//! * `hashbrown-table` - Uses [`hashbrown`] as the internal `HashMap`
//! * `ahasher` - Use [`ahash`]'s `RandomState` as the default hasher
//! * `no-std` - Enables `no_std` + `alloc` support for [`Rodeo`] and [`ThreadedRodeo`]
//!   * Automatically enables the following required features:
//!     * `dashmap/no_std` - `no_std` compatibility for `DashMap`
//!     * `hashbrown-table` - `no_std` `HashMap`
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
//! Rustc: Stable 1.43.1
//!
//! ### Rodeo
//!
//! ### Std's RandomState
//!
//! | Method                       |   Time    |  Throughput  |
//! | :--------------------------- | :-------: | :----------: |
//! | `resolve`                    | 1.9356 μs | 13.214 GiB/s |
//! | `try_resolve`                | 1.9389 μs | 13.191 GiB/s |
//! | `resolve_unchecked`          | 1.4643 μs | 17.467 GiB/s |
//! | `get_or_intern` (empty)      | 95.214 μs | 275.06 MiB/s |
//! | `get_or_intern` (filled)     | 57.163 μs | 458.16 MiB/s |
//! | `try_get_or_intern` (empty)  | 93.504 μs | 280.09 MiB/s |
//! | `try_get_or_intern` (filled) | 57.030 μs | 459.23 MiB/s |
//! | `get` (empty)                | 36.120 μs | 725.08 MiB/s |
//! | `get` (filled)               | 50.915 μs | 514.38 MiB/s |
//!
//! #### AHash's RandomState
//!
//! | Method                       |   Time    |  Throughput  |
//! | :--------------------------- | :-------: | :----------: |
//! | `resolve`                    | 1.9338 μs | 13.226 GiB/s |
//! | `try_resolve`                | 1.9468 μs | 13.137 GiB/s |
//! | `resolve_unchecked`          | 1.4503 μs | 17.635 GiB/s |
//! | `get_or_intern` (empty)      | 56.413 μs | 464.25 MiB/s |
//! | `get_or_intern` (filled)     | 29.770 μs | 879.73 MiB/s |
//! | `try_get_or_intern` (empty)  | 59.106 μs | 443.10 MiB/s |
//! | `try_get_or_intern` (filled) | 31.195 μs | 839.54 MiB/s |
//! | `get` (empty)                | 9.8542 μs | 2.5954 GiB/s |
//! | `get` (filled)               | 23.113 μs | 1.1065 GiB/s |
//!
//! ### FxHash's FxBuildHasher
//!
//! | Method                       |   Time    |  Throughput  |
//! | :--------------------------- | :-------: | :----------: |
//! | `resolve`                    | 2.0569 μs | 12.434 GiB/s |
//! | `try_resolve`                | 1.9505 μs | 13.113 GiB/s |
//! | `resolve_unchecked`          | 1.4477 μs | 17.666 GiB/s |
//! | `get_or_intern` (empty)      | 44.392 μs | 589.97 MiB/s |
//! | `get_or_intern` (filled)     | 27.645 μs | 947.36 MiB/s |
//! | `try_get_or_intern` (empty)  | 43.947 μs | 595.95 MiB/s |
//! | `try_get_or_intern` (filled) | 27.085 μs | 966.95 MiB/s |
//! | `get` (empty)                | 9.4772 μs | 2.6987 GiB/s |
//! | `get` (filled)               | 27.332 μs | 958.20 MiB/s |
//!
//! ### ThreadedRodeo
//!
//! #### Std's RandomState
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :--------------------------- | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`                    |    55.715 μs    |     470.07 MiB/s      |     354.01 μs     |      73.981 MiB/s       |
//! | `try_resolve`                |    55.117 μs    |     475.17 MiB/s      |     380.16 μs     |      68.892 MiB/s       |
//! | `get_or_intern` (empty)      |    282.62 μs    |     92.666 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    103.41 μs    |     253.26 MiB/s      |     433.80 μs     |      60.373 MiB/s       |
//! | `try_get_or_intern` (empty)  |    287.55 μs    |     91.079 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    105.35 μs    |     248.59 MiB/s      |     447.55 μs     |      58.518 MiB/s       |
//! | `get` (empty)                |    86.328 μs    |     303.37 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    95.673 μs    |     273.74 MiB/s      |     465.93 μs     |      56.210 MiB/s       |
//!
//! #### AHash's RandomState
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :--------------------------- | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`                    |    20.103 μs    |     1.2722 GiB/s      |     258.78 μs     |      101.20 MiB/s       |
//! | `try_resolve`                |    17.328 μs    |     1.4760 GiB/s      |     239.13 μs     |      109.52 MiB/s       |
//! | `get_or_intern` (empty)      |    161.98 μs    |     161.68 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    50.065 μs    |     523.11 MiB/s      |     346.60 μs     |      75.563 MiB/s       |
//! | `try_get_or_intern` (empty)  |    159.84 μs    |     163.85 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    51.366 μs    |     509.86 MiB/s      |     331.92 μs     |      78.904 MiB/s       |
//! | `get` (empty)                |    36.637 μs    |     714.84 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    44.606 μs    |     587.13 MiB/s      |     341.70 μs     |      76.645 MiB/s       |
//!
//! #### FxHash's FxBuildHasher
//!
//! | Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :--------------------------- | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`                    |    20.475 μs    |     1.2491 GiB/s      |     230.52 μs     |      113.61 MiB/s       |
//! | `try_resolve`                |    17.479 μs    |     1.4632 GiB/s      |     231.18 μs     |      113.29 MiB/s       |
//! | `get_or_intern` (empty)      |    153.62 μs    |     170.48 MiB/s      |        N\A        |           N\A           |
//! | `get_or_intern` (filled)     |    44.232 μs    |     592.10 MiB/s      |     297.39 μs     |      88.065 MiB/s       |
//! | `try_get_or_intern` (empty)  |    151.58 μs    |     172.78 MiB/s      |        N\A        |           N\A           |
//! | `try_get_or_intern` (filled) |    45.125 μs    |     580.39 MiB/s      |     298.54 μs     |      87.726 MiB/s       |
//! | `get` (empty)                |    33.043 μs    |     792.61 MiB/s      |        N\A        |           N\A           |
//! | `get` (filled)               |    39.044 μs    |     670.78 MiB/s      |     297.38 μs     |      88.068 MiB/s       |
//!
//! ### RodeoReader
//!
//! #### Std's RandomState
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`           |    1.9425 μs    |     13.167 GiB/s      |     4.4657 μs     |      5.7272 GiB/s       |
//! | `resolve_unchecked` |    1.4826 μs    |     17.251 GiB/s      |     3.1239 μs     |      8.1872 GiB/s       |
//! | `try_resolve`       |    1.9535 μs    |     13.092 GiB/s      |     4.1641 μs     |      6.1420 GiB/s       |
//! | `get` (empty)       |    35.895 μs    |     729.62 MiB/s      |     97.991 μs     |      267.27 MiB/s       |
//! | `get` (filled)      |    51.805 μs    |     505.54 MiB/s      |        N\A        |           N\A           |
//!
//! #### AHash's RandomState
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`           |    1.9478 μs    |     13.131 GiB/s      |     4.1532 μs     |      6.1582 GiB/s       |
//! | `resolve_unchecked` |    1.4713 μs    |     17.384 GiB/s      |     3.0922 μs     |      8.2710 GiB/s       |
//! | `try_resolve`       |    1.9584 μs    |     13.059 GiB/s      |     4.2616 μs     |      6.0015 GiB/s       |
//! | `get` (empty)       |    9.9847 μs    |     2.5615 GiB/s      |     48.875 μs     |      535.86 MiB/s       |
//! | `get` (filled)      |    22.848 μs    |     1.1194 GiB/s      |        N\A        |           N\A           |
//!
//! #### FxHash's FxBuildHasher
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`           |    1.9588 μs    |     13.057 GiB/s      |     4.2030 μs     |      6.0852 GiB/s       |
//! | `resolve_unchecked` |    1.4866 μs    |     17.204 GiB/s      |     3.2421 μs     |      7.8886 GiB/s       |
//! | `try_resolve`       |    1.9464 μs    |     13.140 GiB/s      |     4.2429 μs     |      6.0279 GiB/s       |
//! | `get` (empty)       |    9.5245 μs    |     2.6853 GiB/s      |     48.011 μs     |      545.49 MiB/s       |
//! | `get` (filled)      |    27.486 μs    |     952.84 MiB/s      |        N\A        |           N\A           |
//!
//! ### RodeoResolver
//!
//! | Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
//! | :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
//! | `resolve`           |    1.9561 μs    |     13.075 GiB/s      |     4.1818 μs     |      6.1160 GiB/s       |
//! | `resolve_unchecked` |    1.7038 μs    |     15.011 GiB/s      |     3.1031 μs     |      8.2420 GiB/s       |
//! | `try_resolve`       |    1.9490 μs    |     13.123 GiB/s      |     4.3075 μs     |      5.9376 GiB/s       |
//!
//! ### Other Interners (with std's RandomState)
//!
//! | [`string-interner`]      |   Time    |  Throughput  | Relative Perf vs `Rodeo` |
//! | :----------------------- | :-------: | :----------: | :----------------------: |
//! | `resolve`                | 3.8132 μs | 6.7072 GiB/s |         -49.23%          |
//! | `resolve_unchecked`      | 2.3976 μs | 10.667 GiB/s |         -38.92%          |
//! | `get_or_intern` (empty)  | 288.12 μs | 90.899 MiB/s |         -66.95%          |
//! | `get_or_intern` (filled) | 60.104 μs | 435.74 MiB/s |         -5.114%          |
//! | `get` (empty)            | 40.496 μs | 646.72 MiB/s |         -10.80%          |
//! | `get` (filled)           | 63.797 μs | 410.52 MiB/s |         -20.19%          |
//!
//! ### Nightly Benches
//!
//! When the `nightly` feature is enabled, this is the performance you can expect from `Rodeo`.  
//! The functions listed are the ones currently affected by the changes of the `nightly` feature, and the benchmarks were preformed with std's RandomState.  
//! Testing was done on Rust Nightly 1.45.0
//!
//! | Method                       |   Time    |  Throughput  | Relative Perf vs Stable |
//! | :--------------------------- | :-------: | :----------: | :---------------------: |
//! | `get_or_intern` (empty)      | 94.516 μs | 277.09 MiB/s |         +0.73%          |
//! | `get_or_intern` (filled)     | 56.716 μs | 461.77 MiB/s |         +0.78%          |
//! | `try_get_or_intern` (empty)  | 94.629 μs | 276.76 MiB/s |         -1.188%         |
//! | `try_get_or_intern` (filled) | 56.839 μs | 460.77 MiB/s |         +0.336%         |
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
//! [`string-interner`]: https://github.com/Robbepop/string-interner
//! [Nightly Benches]: #nightly-benches

#[macro_use]
mod util;

// mod unique; // Experimental, doesn't currently work
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
