
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
* `no-std` - Enables `no_std` + `alloc` support for [`Rodeo`] and [`ThreadedRodeo`]
  * Automatically enables the following required features:
    * `dashmap/no_std` - Enables `no_std` compatibility for `DashMap`
    * `hashbrown-table` - Required for `no_std` `HashMap`
    * `ahasher` - Required for `no_std` hashing function
* `nightly` - Allows the use of the nightly `hash_raw_entry` feature internally, giving better speed on interning where the key does not yet exist (Only affects `Rodeo`). See [Nightly Benches]

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
Rustc: Stable 1.42.1

### Rodeo

#### Std's RandomState

| Method                       |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `get_or_intern` (empty)      | 210.53 μs | 124.40 MiB/s |
| `get_or_intern` (filled)     | 58.449 μs | 448.06 MiB/s |
| `try_get_or_intern` (empty)  | 240.77 μs | 108.77 MiB/s |
| `try_get_or_intern` (filled) | 58.784 μs | 445.51 MiB/s |
| `get` (empty)                | 37.763 μs | 693.51 MiB/s |
| `get` (filled)               | 51.867 μs | 504.92 MiB/s |
| `resolve`                    | 1.8840 μs | 13.575 GiB/s |
| `try_resolve`                | 1.8828 μs | 13.583 GiB/s |
| `resolve_unchecked`          | 1.4116 μs | 18.117 GiB/s |

#### AHash's RandomState

| Method                       |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `try_get_or_intern` (empty)  | 183.59 μs | 142.65 MiB/s |
| `get_or_intern` (empty)      | 183.57 μs | 142.66 MiB/s |
| `get_or_intern` (filled)     | 29.988 μs | 873.32 MiB/s |
| `try_get_or_intern` (filled) | 30.916 μs | 847.09 MiB/s |
| `get` (empty)                | 10.584 μs | 2.4164 GiB/s |
| `get` (filled)               | 24.760 μs | 1.0329 GiB/s |
| `resolve`                    | 1.8839 μs | 13.576 GiB/s |
| `try_resolve`                | 1.8792 μs | 13.609 GiB/s |
| `resolve_unchecked`          | 1.4121 μs | 18.111 GiB/s |

#### FxHash's FxBuildHasher

| Method                       |   Time    |  Throughput  |
| :--------------------------- | :-------: | :----------: |
| `get_or_intern` (empty)      | 177.51 μs | 147.53 MiB/s |
| `get_or_intern` (filled)     | 32.416 μs | 807.89 MiB/s |
| `try_get_or_intern` (empty)  | 184.99 μs | 141.57 MiB/s |
| `try_get_or_intern` (filled) | 31.387 μs | 834.40 MiB/s |
| `get` (empty)                | 9.3684 μs | 2.7299 GiB/s |
| `get` (filled)               | 25.973 μs | 1008.3 MiB/s |
| `resolve`                    | 1.8825 μs | 13.586 GiB/s |
| `try_resolve`                | 1.8783 μs | 13.616 GiB/s |
| `resolve_unchecked`          | 1.4106 μs | 18.130 GiB/s |

### ThreadedRodeo

#### Std's RandomState

| Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get_or_intern` (empty)      |    471.21 μs    |     55.578 MiB/s      |        N\A        |           N\A           |
| `get_or_intern` (filled)     |    106.87 μs    |     245.05 MiB/s      |     434.95 μs     |      60.211 MiB/s       |
| `try_get_or_intern` (empty)  |    476.41 μs    |     54.971 MiB/s      |        N\A        |           N\A           |
| `try_get_or_intern` (filled) |    107.65 μs    |     243.27 MiB/s      |     470.13 μs     |      55.705 MiB/s       |
| `get` (empty)                |    87.846 μs    |     298.12 MiB/s      |        N\A        |           N\A           |
| `get` (filled)               |    98.844 μs    |     264.95 MiB/s      |     453.81 μs     |      57.709 MiB/s       |
| `resolve`                    |    66.654 μs    |     392.90 MiB/s      |     379.70 μs     |      68.973 MiB/s       |
| `try_resolve`                |    67.401 μs    |     388.56 MiB/s      |     389.46 μs     |      67.244 MiB/s       |

#### AHash's RandomState

| Method                       | Time (1 Thread) | Throughput  (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :--------------------: | :---------------: | :---------------------: |
| `get_or_intern` (empty)      |    355.60 μs    |      73.647 MiB/s      |        N\A        |           N\A           |
| `get_or_intern` (filled)     |    53.505 μs    |      489.47 MiB/s      |     350.98 μs     |      74.616 MiB/s       |
| `try_get_or_intern` (empty)  |    358.33 μs    |      73.085 MiB/s      |        N\A        |           N\A           |
| `try_get_or_intern` (filled) |    54.163 μs    |      483.52 MiB/s      |     375.22 μs     |      69.795 MiB/s       |
| `get` (empty)                |    35.251 μs    |      742.93 MiB/s      |        N\A        |           N\A           |
| `get` (filled)               |    46.591 μs    |      562.11 MiB/s      |     352.05 μs     |      74.390 MiB/s       |
| `resolve`                    |    19.550 μs    |      1.3082 GiB/s      |     283.64 μs     |      92.333 MiB/s       |
| `try_resolve`                |    16.892 μs    |      1.5140 GiB/s      |     239.45 μs     |      109.37 MiB/s       |

#### FxHash's FxBuildHasher

| Method                       | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :--------------------------- | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get_or_intern` (empty)      |    343.93 μs    |     76.146 MiB/s      |        N\A        |           N\A           |
| `get_or_intern` (filled)     |    45.682 μs    |     573.28 MiB/s      |     313.56 μs     |      83.522 MiB/s       |
| `try_get_or_intern` (empty)  |    323.60 μs    |     80.930 MiB/s      |        N\A        |           N\A           |
| `try_get_or_intern` (filled) |    46.621 μs    |     561.74 MiB/s      |     304.91 μs     |      85.890 MiB/s       |
| `get` (empty)                |    32.313 μs    |     810.47 MiB/s      |        N\A        |           N\A           |
| `get` (filled)               |    40.448 μs    |     647.47 MiB/s      |     318.43 μs     |      82.244 MiB/s       |
| `resolve`                    |    23.790 μs    |     1.0750 GiB/s      |     262.68 μs     |      99.698 MiB/s       |
| `try_resolve`                |    22.759 μs    |     1.1238 GiB/s      |     244.69 μs     |      107.03 MiB/s       |

### RodeoReader

#### Std's RandomState

| Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    35.523 μs    |     737.24 MiB/s      |        N/A        |           N/A           |
| `get` (filled)      |    47.964 μs    |     546.01 MiB/s      |     91.646 μs     |      285.76 MiB/s       |
| `resolve`           |    1.8889 μs    |     13.539 GiB/s      |     4.1066 μs     |      6.2278 GiB/s       |
| `try_resolve`       |    1.8749 μs    |     13.641 GiB/s      |     4.1582 μs     |      6.1506 GiB/s       |
| `resolve_unchecked` |    1.4135 μs    |     18.093 GiB/s      |     3.0388 μs     |      8.4163 GiB/s       |

#### AHash's RandomState

| Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    9.7012 μs    |     2.6363 GiB/s      |        N/A        |           N/A           |
| `get` (filled)      |    23.678 μs    |     1.0801 GiB/s      |     49.281 μs     |      531.42 MiB/s       |
| `resolve`           |    1.8859 μs    |     13.561 GiB/s      |     4.1415 μs     |      6.1753 GiB/s       |
| `try_resolve`       |    1.8790 μs    |     13.611 GiB/s      |     4.4200 μs     |      5.7862 GiB/s       |
| `resolve_unchecked` |    1.4104 μs    |     18.133 GiB/s      |     3.0900 μs     |      8.2766 GiB/s       |

#### FxHash's FxBuildHasher

| Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `get` (empty)       |    8.9661 μs    |     2.8524 GiB/s      |        N/A        |           N/A           |
| `get` (filled)      |    21.608 μs    |     1.1836 GiB/s      |     46.744 μs     |      560.26 MiB/s       |
| `resolve`           |    1.8769 μs    |     13.626 GiB/s      |     4.1747 μs     |      6.1261 GiB/s       |
| `try_resolve`       |    1.8799 μs    |     13.604 GiB/s      |     4.1995 μs     |      6.0900 GiB/s       |
| `resolve_unchecked` |    1.4092 μs    |     18.149 GiB/s      |     3.0744 μs     |      8.3188 GiB/s       |

### RodeoResolver

| Method              | Time (1 Thread) | Throughput (1 Thread) | Time (24 Threads) | Throughput (24 Threads) |
| :------------------ | :-------------: | :-------------------: | :---------------: | :---------------------: |
| `resolve`           |    1.9122 μs    |     13.375 GiB/s      |     3.9084 μs     |      6.5436 GiB/s       |
| `try_resolve`       |    1.9077 μs    |     13.406 GiB/s      |     3.8922 μs     |      6.5709 GiB/s       |
| `resolve_unchecked` |    1.6634 μs    |     15.375 GiB/s      |     3.1002 μs     |      8.2494 GiB/s       |

### Other Interners (with std's RandomState)

| [`string-interner`]      |   Time    |  Throughput  | Relative Perf vs `Rodeo` |
| :----------------------- | :-------: | :----------: | :----------------------: |
| `get_or_intern` (empty)  | 278.41 μs | 94.065 MiB/s |         -32.24%          |
| `get_or_intern` (filled) | 58.421 μs | 448.28 MiB/s |          -0.04%          |
| `get` (empty)            | 38.991 μs | 671.66 MiB/s |          -3.25%          |
| `get` (filled)           | 58.708 μs | 446.09 MiB/s |         -13.18%          |
| `resolve`                | 2.7759 μs | 9.2133 GiB/s |         -47.34%          |
| `resolve_unchecked`      | 2.3413 μs | 10.924 GiB/s |         -65.86%          |

### Nightly Benches

When the `nightly` feature is enabled, this is the performance you can expect from `Rodeo`.  
The functions listed are the ones currently affected by the changes of the `nightly` feature, and the benchmarks were preformed with std's RandomState.  
Testing was done on Rust Nightly v1.43.0

| Method                       |   Time    |  Throughput  | Relative Perf vs Stable |
| :--------------------------- | :-------: | :----------: | :---------------------: |
| `get_or_intern` (empty)      | 182.60 μs | 143.42 MiB/s |         +13.26%         |
| `get_or_intern` (filled)     | 54.864 μs | 477.34 MiB/s |         +6.53%          |
| `try_get_or_intern` (empty)  | 206.10 μs | 127.07 MiB/s |         +16.82%         |
| `try_get_or_intern` (filled) | 52.073 μs | 502.93 MiB/s |         +12.88%         |

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
[Nightly Benches]: #nightly-benches
