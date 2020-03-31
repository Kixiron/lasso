mod setup;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use setup::{StringInternerEmptySetup, StringInternerFilledSetup, INPUT};

fn interner_std(c: &mut Criterion) {
    use std::collections::hash_map::RandomState;

    let mut group = c.benchmark_group("string-interner (std)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = StringInternerEmptySetup::new(RandomState::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |mut interner| {
                for &line in setup.lines() {
                    black_box(interner.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_interner_mut().get_or_intern(line));
            }
        })
    });

    let setup = StringInternerEmptySetup::new(RandomState::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    black_box(interner.resolve(*key));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    unsafe { black_box(interner.resolve_unchecked(*key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

fn interner_ahash(c: &mut Criterion) {
    use ahash::RandomState;

    let mut group = c.benchmark_group("string-interner (ahash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = StringInternerEmptySetup::new(RandomState::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |mut interner| {
                for &line in setup.lines() {
                    black_box(interner.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_interner_mut().get_or_intern(line));
            }
        })
    });

    let setup = StringInternerEmptySetup::new(RandomState::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    black_box(interner.resolve(*key));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(RandomState::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    unsafe { black_box(interner.resolve_unchecked(*key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

fn interner_fxhash(c: &mut Criterion) {
    use fxhash::FxBuildHasher;

    let mut group = c.benchmark_group("string-interner (fxhash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = StringInternerEmptySetup::new(FxBuildHasher::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |mut interner| {
                for &line in setup.lines() {
                    black_box(interner.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = StringInternerFilledSetup::new(FxBuildHasher::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_interner_mut().get_or_intern(line));
            }
        })
    });

    let setup = StringInternerEmptySetup::new(FxBuildHasher::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(FxBuildHasher::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for &line in setup.lines() {
                    black_box(interner.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(FxBuildHasher::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    black_box(interner.resolve(*key).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = StringInternerFilledSetup::new(FxBuildHasher::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_interner(),
            |interner| {
                for key in setup.keys() {
                    unsafe { black_box(interner.resolve_unchecked(*key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

criterion_group!(benches, interner_std, interner_ahash, interner_fxhash);
criterion_main!(benches);
