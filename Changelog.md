# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [0.3.0] - 2020-07-12

This version really wouldn't be possible without the amazing work of @CAD97. They were an amazing asset in [improving the memory efficiency](https://github.com/Kixiron/lasso/issues/4) of lasso and I can't thank them enough

### Added

- Added the `get_or_intern_static` and `try_get_or_intern_static` methods for zero-copy static string internment
- Added the `Capacity` struct for better internment estimates and more accurate pre-allocation

### Changed

- Lasso's single-threaded configuration now supports back to Rust 1.34! Thanks to @jyn514 for their [hard work](https://github.com/Kixiron/lasso/pull/3)!
- `Rodeo` and `RodeoReader` now use less memory since they only store their interned strings' pointers once
- `Rodeo` and `RodeoReader` use a different hashing strategy for their maps, instead of using a hashed string and key pair, they now use the key hashed *as* their paired string. This allows for decreased memory usage
- The arena backing all interners now increases the amount of memory it allocates exponentially (The same doubling strategy used by `Vec` is used). This allows for fewer allocations to happen as more strings are interned
- `hashbrown` is now a default dependency due to `HashMap`'s `raw_api` not being stable
- Relaxed trait bounds of many structs and functions
- Made custom `Debug` implementations to cut the excess and unneeded output
- Exported the `Strings` and `Iter` structs
- The `with_capacity` methods now use the `Capacity` struct

### Removed

- Removed the `hashbrown-table` and `nightly` features

## 0.2.4

### Added

- Added Serde support with the `serialize` feature

## 0.2.2

### Fixed

- Fixed `Send` for `Rodeo`

## 0.2.0

### Added

- Added single-threaded interner
- Added `try_get_or_intern`
- Added feature for `hashbrown`
- Added feature for `parking_lot`
- Added `no-std` feature
- Added `Key::try_from_usize`
- Added `MiniSpur`
- Added `MicroCord`
- Removed blanket impls for `u8`-`usize` & the nonzero  varieties
- Added lifetimes to all `Rodeo` types
- Added lifetime to `Key`
- Added the ID requirement to `Key`
- Added `try_resolve`s and `resolve_unchecked`s
- Added `strings()` and `iter()` methods to `Rodeo`, `RodeoResolver` and `RodeoReader`
- Strings are now allocated via an arena allocator

### Changed

- Renamed `Lasso` to `ThreadedRodeo`
- Renamed `ReadOnlyLasso` to `RodeoReader`
- Renamed `ResolverLasso` to `RodeoResolver`
- Changed default impl of `Key::from_usize`
- Added `Send` and `Sync` bounds for `ThreadedRodeo`, `RodeoResolver` and `RodeoReader`
- Changed internals of `get_or_intern` to be `try_get_or_intern.expect()`
- `multi-threaded` is now actually disabled by default

### Removed

- Removed `.intern` from all structs
- `Rodeo` and `ThreadedRodeo` no longer implement `Clone`

### Fixed

- Fixed memory leaks and possible unsoundness
- Fixed data races on `ThreadedRodeo`
- Fixed memory leaks in `ThreadedRodeo`, for real this time

## 0.1.2
## 0.1.1
## 0.1.0

<!-- next-url -->
[Unreleased]: https://github.com/Kixiron/lasso/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Kixiron/lasso/compare/v0.3.0
