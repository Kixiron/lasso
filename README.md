
[![CI][1]][0]
[![Security Audit][2]][0]
[![Coverage][3]][4]
[![LoC][5]][0]
[![Docs.rs][6]][7]
[![Crates.io][8]][9]

A multithreaded and single threaded string interner that allows strings to be cached with a minimal memory footprint,
associating them with a unique [key] that can be used to retrieve them at any time. A [`Rodeo`] allows `O(1)`
internment and resolution and can be turned into a [`RodeoReader`] to allow for contention-free resolutions
with both key to str and str to key operations. It can also be turned into a [`RodeoResolver`] with only
key to str operations for the lowest possible memory usage.

## Which interner do I use?

For single-threaded workloads [`Rodeo`] is encouraged, while multi-threaded applications should use [`ThreadedRodeo`].
Both of these are the only way to intern strings, but most applications will hit a stage where they are done interning
strings, and at that point is where the choice between [`RodeoReader`] and [`RodeoResolver`]. If the user needs to get
keys for strings still, then they must use the [`RodeoReader`] (although they can still transfer into a  [`RodeoResolver`])
at this point. For users who just need key to string resolution, the [`RodeoResolver`] gives contention-free access at the
minimum possible memory usage. Note that to gain access to [`ThreadedRodeo`] the `multi-threaded` feature is required.

| Interner          | Thread-safe | Intern String | str to key | key to str | Contention Free | Memory Usage |
| ----------------- | :---------: | :-----------: | :--------: | :--------: | :-------------: | :----------: |
| [`Rodeo`]         |      ❌      |       ✅       |     ✅      |     ✅      |       N/A       |    Medium    |
| [`ThreadedRodeo`] |      ✅      |       ✅       |     ✅      |     ✅      |        ❌        |     Most     |
| [`RodeoReader`]   |      ✅      |       ❌       |     ✅      |     ✅      |        ✅        |    Medium    |
| [`RodeoResolver`] |      ✅      |       ❌       |     ❌      |     ✅      |        ✅        |    Least     |

## Cargo Features

By default `lasso` has zero dependencies only the [`Rodeo`] is exposed. To make use of [`ThreadedRodeo`], you must enable the `multi-threaded` feature

* `multi-threaded` - Enables [`ThreadedRodeo`], the interner for multi-threaded tasks
* `hashbrown-table` - Uses [`hashbrown`] as the internal `HashMap`
* `ahasher` - Use [`ahash`]'s `RandomState` as the default hasher
* `no_std` - Enables `no_std` + `alloc` support for [`Rodeo`] and [`ThreadedRodeo`]
  * Automatically enables the following required features:
    * `dashmap/no_std` - Enables `no_std` compatibility for `DashMap`
    * `hashbrown-table` - Required for `no_std` `HashMap`
    * `ahasher` - Required for `no_std` hashing function
* `nightly` - Allows the use of the nightly `hash_raw_entry` feature internally, giving better speed on interning where the key does not yet exist (Only affects `Rodeo`).

## Example: Using Rodeo

```rust
use lasso::Rodeo;

let mut rodeo = Rodeo::default();
let key = rodeo.get_or_intern("Hello, world!");

// Easily retrieve the value of a key and find the key for values
assert_eq!("Hello, world!", rodeo.resolve(&key));
assert_eq!(Some(key), rodeo.get("Hello, world!"));

// Interning the same string again will yield the same key
let key2 = rodeo.get_or_intern("Hello, world!");

assert_eq!(key, key2);
```

## Example: Using ThreadedRodeo

```rust
use lasso::ThreadedRodeo;
use std::{thread, sync::Arc};

let rodeo = Arc::new(ThreadedRodeo::default());
let key = rodeo.get_or_intern("Hello, world!");

// Easily retrieve the value of a key and find the key for values
assert_eq!("Hello, world!", rodeo.resolve(&key));
assert_eq!(Some(key), rodeo.get("Hello, world!"));

// Interning the same string again will yield the same key
let key2 = rodeo.get_or_intern("Hello, world!");

assert_eq!(key, key2);

// ThreadedRodeo can be shared across threads
let moved = Arc::clone(&rodeo);
let hello = thread::spawn(move || {
    assert_eq!("Hello, world!", moved.resolve(&key));
    moved.get_or_intern("Hello from the thread!")
})
.join()
.unwrap();

assert_eq!("Hello, world!", rodeo.resolve(&key));
assert_eq!("Hello from the thread!", rodeo.resolve(&hello));
```

## Example: Creating a RodeoReader

```rust
use lasso::Rodeo;

// Rodeo and ThreadedRodeo are interchangeable here
let mut rodeo = Rodeo::default();

let key = rodeo.get_or_intern("Hello, world!");
assert_eq!("Hello, world!", rodeo.resolve(&key));

let reader = rodeo.into_reader();

// Reader keeps all the strings from the parent
assert_eq!("Hello, world!", reader.resolve(&key));
assert_eq!(Some(key), reader.get("Hello, world!"));

// The Reader can now be shared across threads, no matter what kind of Rodeo created it
```

## Example: Creating a RodeoResolver

```rust
use lasso::Rodeo;

// Rodeo and ThreadedRodeo are interchangeable here
let mut rodeo = Rodeo::default();

let key = rodeo.get_or_intern("Hello, world!");
assert_eq!("Hello, world!", rodeo.resolve(&key));

let resolver = rodeo.into_resolver();

// Resolver keeps all the strings from the parent
assert_eq!("Hello, world!", resolver.resolve(&key));

// The Resolver can now be shared across threads, no matter what kind of Rodeo created it
```

## Benchmarks

Benchmarks were gathered with [Criterion.rs](https://github.com/bheisler/criterion.rs)  
OS: Windows 10  
CPU: Ryzen 9 3900X at 3800Mhz  
RAM: 3200Mhz  

### Rodeo

#### Std's RandomState

| Function                     |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `get_or_intern` (empty)      | 208.30 μs | 125.37 MiB/s |
| `get_or_intern` (filled)     | 51.307 μs | 509.01 MiB/s |
| `try_get_or_intern` (empty)  | 241.64 μs | 108.07 MiB/s |
| `try_get_or_intern` (filled) | 52.351 μs | 498.86 MiB/s |
| `get` (empty)                | 34.923 μs | 747.81 MiB/s |
| `get` (filled)               | 51.252 μs | 509.54 MiB/s |
| `resolve`                    | 1.9273 μs | 13.233 GiB/s |
| `try_resolve`                | 1.9048 μs | 13.389 GiB/s |
| `resolve_unchecked`          | 1.4406 μs | 17.703 GiB/s |

#### AHash's RandomState

| Function                     |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `get_or_intern` (empty)      | 178.66 μs | 146.17 MiB/s |
| `get_or_intern` (filled)     | 23.901 μs | 1.0670 GiB/s |
| `try_get_or_intern` (empty)  | 189.67 μs | 137.69 MiB/s |
| `try_get_or_intern` (filled) | 23.832 μs | 1.0701 GiB/s |
| `get` (empty)                | 10.038 μs | 2.5406 GiB/s |
| `get` (filled)               | 24.263 μs | 1.0511 GiB/s |
| `resolve`                    | 1.9088 μs | 13.361 GiB/s |
| `try_resolve`                | 1.9109 μs | 13.346 GiB/s |
| `resolve_unchecked`          | 1.4304 μs | 17.830 GiB/s |

#### FxHash's FxBuildHasher

| Function                     |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `get_or_intern` (empty)      | 185.15 μs | 141.05 MiB/s |
| `get_or_intern` (filled)     | 26.964 μs | 968.52 MiB/s |
| `try_get_or_intern` (empty)  | 158.10 μs | 165.19 MiB/s |
| `try_get_or_intern` (filled) | 25.853 μs | 1010.2 MiB/s |
| `get` (empty)                | 9.9517 μs | 2.5627 GiB/s |
| `get` (filled)               | 27.211 μs | 959.75 MiB/s |
| `resolve`                    | 1.9118 μs | 13.340 GiB/s |
| `try_resolve`                | 1.9027 μs | 13.404 GiB/s |
| `resolve_unchecked`          | 1.4439 μs | 17.663 GiB/s |

### ThreadedRodeo

#### Std's RandomState

| Function                     | Time (1 thread) | Throughput (1 Threads) | Time (24 thread) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :--------------------: | :--------------: | :---------------------: |
| `get_or_intern` (empty)      |    1.6771 ms    |      15.616 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get_or_intern` (filled)     |    107.48 μs    |      243.67 MiB/s      |    442.31 μs     |      59.209 MiB/s       |
| `try_get_or_intern` (empty)  |    1.7059 ms    |      15.352 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `try_get_or_intern` (filled) |    106.01 μs    |      247.04 MiB/s      |    432.66 μs     |      60.530 MiB/s       |
| `get` (empty)                |    85.732 μs    |      305.47 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)               |    99.197 μs    |      264.01 MiB/s      |    459.45 μs     |      57.000 MiB/s       |
| `resolve`                    |    67.733 μs    |      386.65 MiB/s      |    398.22 μs     |      65.766 MiB/s       |
| `try_resolve`                |    69.947 μs    |      374.41 MiB/s      |    365.34 μs     |      71.683 MiB/s       |

#### AHash's RandomState

| Function                     | Time (1 thread) | Throughput (1 Threads) | Time (24 thread) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :--------------------: | :--------------: | :---------------------: |
| `get_or_intern` (empty)      |    1.5925 ms    |      16.445 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get_or_intern` (filled)     |    55.865 μs    |      468.79 MiB/s      |    365.92 μs     |      71.569 MiB/s       |
| `try_get_or_intern` (empty)  |    1.5711 ms    |      16.669 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `try_get_or_intern` (filled) |    56.809 μs    |      461.00 MiB/s      |    375.74 μs     |      69.699 MiB/s       |
| `get` (empty)                |    41.242 μs    |      635.01 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)               |    49.824 μs    |      525.62 MiB/s      |    342.02 μs     |      76.571 MiB/s       |
| `resolve`                    |    19.841 μs    |      1.2890 GiB/s      |    256.48 μs     |      102.11 MiB/s       |
| `try_resolve`                |    17.193 μs    |      1.4876 GiB/s      |    254.14 μs     |      103.05 MiB/s       |

#### FxHash's FxBuildHasher

| Function                     | Time (1 thread) | Throughput (1 Threads) | Time (24 thread) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :--------------------: | :--------------: | :---------------------: |
| `get_or_intern` (empty)      |    1.5497 ms    |      16.899 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get_or_intern` (filled)     |    48.920 μs    |      535.34 MiB/s      |    295.52 μs     |      88.621 MiB/s       |
| `try_get_or_intern` (empty)  |    1.5593 ms    |      16.795 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `try_get_or_intern` (filled) |    48.075 μs    |      544.75 MiB/s      |    300.31 μs     |      87.206 MiB/s       |
| `get` (empty)                |    33.225 μs    |      788.22 MiB/s      | N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)               |    42.019 μs    |      623.26 MiB/s      |    299.04 μs     |      87.578 MiB/s       |
| `resolve`                    |    24.380 μs    |      1.0490 GiB/s      |    241.09 μs     |      108.63 MiB/s       |
| `try_resolve`                |    22.798μs     |      1.1218 GiB/s      |    256.45 μs     |      102.12 MiB/s       |

<sup>1</sup> Tested with filled `ThreadedRodeo`, empty was infeasible to accurately test

### RodeoReader

#### Std's RandomState

| Function            | Time (1 thread) | Throughput (1 thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    38.031 μs    |     686.68 MiB/s      |  N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)      |    51.326 μs    |     508.81 MiB/s      |     99.161 μs     |      263.36 MiB/s       |
| `resolve`           |    1.9029 μs    |     13.402 GiB/s      |     4.1835 μs     |      6.0962 GiB/s       |
| `try_resolve`       |    1.9071 μs    |     13.373 GiB/s      |     4.2855 μs     |      5.9511 GiB/s       |
| `resolve_unchecked` |    1.4313 μs    |     17.819 GiB/s      |     3.1906 μs     |      7.9932 GiB/s       |

#### AHash's RandomState

| Function            | Time (1 thread) | Throughput (1 thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    9.6449 μs    |     2.6442 GiB/s      |  N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)      |    23.971 μs    |     1.0639 GiB/s      |     55.763 μs     |      468.33 MiB/s       |
| `resolve`           |    1.8999 μs    |     13.424 GiB/s      |     4.2253 μs     |      6.0359 GiB/s       |
| `try_resolve`       |    1.9008 μs    |     13.417 GiB/s      |     4.3028 μs     |      5.9272 GiB/s       |
| `resolve_unchecked` |    1.4319 μs    |     17.810 GiB/s      |     3.1734 μs     |      8.0367 GiB/s       |

#### FxHash's FxBuildHasher 

| Function            | Time (1 thread) | Throughput (1 thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    9.7933 μs    |     2.6042 GiB/s      |  N\A<sup>1</sup>  |     N\A<sup>1</sup>     |
| `get` (filled)      |    26.739 μs    |     976.67 MiB/s      |     48.247 μs     |      541.28 MiB/s       |
| `resolve`           |    1.9003 μs    |     13.421 GiB/s      |     4.2417 μs     |      6.0125 GiB/s       |
| `try_resolve`       |    1.9030 μs    |     13.401 GiB/s      |     4.2682 μs     |      5.9752 GiB/s       |
| `resolve_unchecked` |    1.4374 μs    |     17.743 GiB/s      |     3.2082 μs     |      7.9495 GiB/s       |

<sup>1</sup> Tested with filled `RodeoReader`, empty was infeasible to accurately test

### RodeoResolver

| Function            | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `resolve`           |    1.9213 μs    |     13.274 GiB/s      |     3.8982 μs     |      6.5423 GiB/s       |
| `try_resolve`       |    1.9171 μs    |     13.303 GiB/s      |     3.9563 μs     |      6.4462 GiB/s       |
| `resolve_unchecked` |    1.6777 μs    |     15.202 GiB/s      |     3.0775 μs     |      8.2870 GiB/s       |

### Other Interners

Other interners were tested with std's RandomState. Relative performance was calculated with `((other - rodeo) / |rodeo|) * 100%`

| [`string-interner`]      |   Time    |  Throughput  | Relative Perf (vs `Rodeo`) |
| :----------------------- | :-------: | :----------: | :------------------------: |
| `get_or_intern` (empty)  | 304.24 μs | 85.839 MiB/s |           -46.0%           |
| `get_or_intern` (filled) | 62.462 μs | 418.10 MiB/s |           -21.7%           |
| `get` (empty)            | 39.794 μs | 656.26 MiB/s |           -13.9%           |
| `get` (filled)           | 62.434 μs | 418.29 MiB/s |           -21.8%           |
| `resolve`                | 2.8477 μs | 8.9559 GiB/s |           -47.7%           |
| `resolve_unchecked`      | 2.3829 μs | 10.703 GiB/s |           -65.4%           |

[0]: https://github.com/Kixiron/lasso
[1]: https://github.com/Kixiron/lasso/workflows/Build/badge.svg
[2]: https://github.com/Kixiron/lasso/workflows/Security%20Audit/badge.svg
[3]: https://coveralls.io/repos/github/Kixiron/lasso/badge.svg?branch=master
[4]: https://coveralls.io/github/Kixiron/lasso?branch=master
[5]: https://tokei.rs/b1/github/Kixiron/lasso
[6]: https://docs.rs/lasso/badge.svg
[7]: https://docs.rs/lasso
[8]: https://img.shields.io/crates/v/lasso.svg
[9]: https://crates.io/crates/lasso
[key]: crate::Key
[`Rodeo`]: crate::Rodeo
[`ThreadedRodeo`]: crate::ThreadedRodeo
[`RodeoResolver`]: crate::RodeoResolver
[`RodeoReader`]: crate::RodeoReader
[`hashbrown`]: https://crates.io/crates/hashbrown
[`ahash`]: https://crates.io/crates/ahash
[`parking_lot`]: https://crates.io/crates/parking_lot
[`string-interner`]: https://github.com/Robbepop/string-interner
