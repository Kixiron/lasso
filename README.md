# Lasso

A concurrent string interner that allows strings to be cached with a minimal memory footprint,
associating them with a unique [key] that can be used to retrieve them at any time. [`Lassos`] allow `O(1)`
internment and resolution and can be turned into a [`ReadOnlyLasso`] to allow for contention-free resolutions.

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

assert_eq!(Some("Hello, "), lasso.resolve(&hello));
assert_eq!(Some("World!"), lasso.resolve(&world));

// These are the same because they interned the same string
assert_eq!(world, world_2);
assert_eq!(lasso.resolve(&world), lasso.resolve(&world_2));
```

## Example: Resolving Strings

```rust
use lasso::Lasso;

let lasso = Lasso::default();
let key = lasso.intern("Hello, World!");

assert_eq!(Some("Hello, World!"), lasso.resolve(&key));
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
    assert_eq!(Some("Contention free!"), lasso.resolve(&key));
});

assert_eq!(Some("Contention free!"), read_only_lasso.resolve(&key));
```

# Cargo Features

* `default` - By default the `ahasher` feature is enabled
* `ahasher` - Use [`ahash::RandomState`] as the default hasher for extra speed, without this then std's [`RandomState`] will be used

[key]: crate::Key
[`Lassos`]: crate::Lasso
[`ReadOnlyLasso`]: crate::ReadOnlyLasso
[`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/
[`RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
