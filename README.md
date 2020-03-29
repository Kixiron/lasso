# Lasso

[![CI](https://github.com/Kixiron/lasso/workflows/CI/badge.svg)](https://github.com/Kixiron/lasso)
[![Security Audit](https://github.com/Kixiron/lasso/workflows/Security%20Audit/badge.svg)](https://github.com/Kixiron/lasso)
[![Coverage](https://coveralls.io/repos/github/Kixiron/lasso/badge.svg?branch=master)](https://coveralls.io/github/Kixiron/lasso?branch=master)
[![LoC](https://tokei.rs/b1/github/Kixiron/lasso)](https://github.com/Kixiron/lasso)
[![Docs.rs](https://docs.rs/lasso/badge.svg)](https://docs.rs/lasso)
[![Crates.io](https://img.shields.io/crates/v/lasso.svg)](https://crates.io/crates/lasso)

A concurrent string interner that allows strings to be cached with a minimal memory footprint,
associating them with a unique [key] that can be used to retrieve them at any time. [`Lassos`] allow `O(1)`
internment and resolution and can be turned into a [`ReadOnlyLasso`] to allow for contention-free resolutions
with both key to str and str to key operations. It can also be turned into a [`ResolverLasso`] with only
key to str operations for the lowest possible memory usage

## Which Interner do I use?

No matter which interner you decide to use, you must start with a [`Lasso`], as that is the only way to intern strings and
the only way to get the other two interner types. As your program progresses though you may not need to intern strings anymore,
and at that point you may choose either a [`ReadOnlyLasso`] or a [`ResolverLasso`]. If you need to go from str to key you
should use a [`ReadOnlyLasso`], but anything else should use a [`ResolverLasso`].

| Interner          | Thread-safe | Intern String | key to str | str to key | Contention Free | Relative Memory Usage |
| ----------------- | :---------: | :-----------: | :--------: | :--------: | :-------------: | :-------------------: |
| [`Lasso`]         |      ✅      |       ✅       |     ✅      |     ✅      |        ❌        |         Most          |
| [`ReadOnlyLasso`] |      ✅      |       ❌       |     ✅      |     ✅      |        ✅        |        Middle         |
| [`ResolverLasso`] |      ✅      |       ❌       |     ✅      |     ❌      |        ✅        |         Least         |

## Example: Interning Strings across threads

```rust
use lasso::Lasso;
use std::{thread, sync::Arc};

let lasso = Arc::new(Lasso::default());
let hello = lasso.get_or_intern("Hello, ");

let l = Arc::clone(&lasso);
let world = thread::spawn(move || {
    l.get_or_intern("World!".to_string())
})
.join()
.unwrap();

let world_2 = lasso.get_or_intern("World!");

assert_eq!("Hello, ", lasso.resolve(&hello));
assert_eq!("World!", lasso.resolve(&world));

// These are the same because they interned the same string
assert_eq!(world, world_2);
assert_eq!(lasso.resolve(&world), lasso.resolve(&world_2));
```

## Example: Resolving Strings

```rust
use lasso::Lasso;

let lasso = Lasso::default();
let key = lasso.intern("Hello, World!");

assert_eq!("Hello, World!", lasso.resolve(&key));
```

## Example: Creating a ReadOnlyLasso

```rust
use lasso::Lasso;
use std::{thread, sync::Arc};

let lasso = Lasso::default();
let key = lasso.intern("Contention free!");

// Can be used for resolving strings with zero contention, but not for interning new ones
let read_only_lasso = Arc::new(lasso.into_read_only());

let lasso = Arc::clone(&read_only_lasso);
thread::spawn(move || {
    assert_eq!("Contention free!", lasso.resolve(&key));
});

assert_eq!("Contention free!", read_only_lasso.resolve(&key));
```

## Example: Creating a ResolverLasso

```rust
use lasso::Lasso;
use std::{thread, sync::Arc};

let lasso = Lasso::default();
let key = lasso.intern("Contention free!");

// Can be used for resolving strings with zero contention and the lowest possible memory consumption,
// but not for interning new ones
let resolver_lasso = Arc::new(lasso.into_resolver());

let lasso = Arc::clone(&resolver_lasso);
thread::spawn(move || {
    assert_eq!("Contention free!", lasso.resolve(&key));
});

assert_eq!("Contention free!", resolver_lasso.resolve(&key));
```

## Cargo Features

* `default` - By default the `ahasher` feature is enabled
* `ahasher` - Use [`ahash::RandomState`] as the default hasher for extra speed, without this then std's [`RandomState`] will be used

[key]: crate::Key
[`Lasso`]: crate::Lasso
[`Lassos`]: crate::Lasso
[`ReadOnlyLasso`]: crate::ReadOnlyLasso
[`ResolverLasso`]: crate::ResolverLasso
[`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/
[`RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
