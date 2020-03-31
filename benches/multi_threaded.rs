mod setup;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use setup::{
    run_threaded_filled, ThreadedRodeoEmptySetup, ThreadedRodeoFilledSetup, INPUT, NUM_THREADS,
};

// TODO: Benchmark all of Rodeo's functions & benchmark ThreadedRodeo, RodeoReader and RodeoResolver

fn rodeo_std(c: &mut Criterion) {
    use std::collections::hash_map::RandomState;

    let mut group = c.benchmark_group("threaded rodeo (std)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().get_or_intern(line));
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("try_get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.try_get_or_intern(line).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("try_get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().try_get_or_intern(line).unwrap());
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.resolve(key));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("try_resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.try_resolve(key).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    unsafe { black_box(rodeo.resolve_unchecked(key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

fn rodeo_std_threaded(c: &mut Criterion) {
    use std::collections::hash_map::RandomState;

    let mut group = c.benchmark_group("threaded rodeo w/threads (std)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    group.bench_function("get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get_or_intern(line));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("try_get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.try_get_or_intern(line).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("get", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get(line));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.resolve(key));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("try_resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.try_resolve(key).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("resolve_unchecked", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        unsafe { black_box(rodeo.resolve_unchecked(key)) };
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.finish();
}

fn rodeo_ahash(c: &mut Criterion) {
    use ahash::RandomState;

    let mut group = c.benchmark_group("threaded rodeo (ahash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().get_or_intern(line));
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("try_get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.try_get_or_intern(line).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("try_get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().try_get_or_intern(line).unwrap());
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(RandomState::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.resolve(key));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("try_resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.try_resolve(key).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(RandomState::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    unsafe { black_box(rodeo.resolve_unchecked(key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

fn rodeo_ahash_threaded(c: &mut Criterion) {
    use ahash::RandomState;

    let mut group = c.benchmark_group("threaded rodeo w/threads (ahash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    group.bench_function("get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get_or_intern(line));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("try_get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.try_get_or_intern(line).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("get", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get(line));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.resolve(key));
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("try_resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.try_resolve(key).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.bench_function("resolve_unchecked", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        unsafe { black_box(rodeo.resolve_unchecked(key)) };
                    }
                },
                NUM_THREADS,
                iters,
                RandomState::new(),
            )
        })
    });

    group.finish();
}

fn rodeo_fxhash(c: &mut Criterion) {
    use fxhash::FxBuildHasher;

    let mut group = c.benchmark_group("threaded rodeo w/ threads (fxhash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = ThreadedRodeoEmptySetup::new(FxBuildHasher::default());
    group.bench_function("get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get_or_intern(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().get_or_intern(line));
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(FxBuildHasher::default());
    group.bench_function("try_get_or_intern (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.try_get_or_intern(line).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let mut setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("try_get_or_intern (filled)", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().try_get_or_intern(line).unwrap());
            }
        })
    });

    let setup = ThreadedRodeoEmptySetup::new(FxBuildHasher::default());
    group.bench_function("get (empty)", |b| {
        b.iter_batched(
            || setup.empty_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("get (filled)", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for &line in setup.lines() {
                    black_box(rodeo.get(line));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.resolve(key));
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("try_resolve", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    black_box(rodeo.try_resolve(key).unwrap());
                }
            },
            BatchSize::PerIteration,
        )
    });

    let setup = ThreadedRodeoFilledSetup::new(FxBuildHasher::default());
    group.bench_function("resolve_unchecked", |b| {
        b.iter_batched(
            || setup.filled_rodeo(),
            |rodeo| {
                for key in setup.keys() {
                    unsafe { black_box(rodeo.resolve_unchecked(key)) };
                }
            },
            BatchSize::PerIteration,
        )
    });

    group.finish();
}

fn rodeo_fxhash_threaded(c: &mut Criterion) {
    use fxhash::FxBuildHasher;

    let mut group = c.benchmark_group("threaded rodeo w/threads (fxhash)");
    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    group.bench_function("get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get_or_intern(line));
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.bench_function("try_get_or_intern", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.try_get_or_intern(line).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.bench_function("get", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, _| {
                    for &line in setup::bench_lines() {
                        black_box(rodeo.get(line));
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.bench_function("resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.resolve(key));
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.bench_function("try_resolve", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        black_box(rodeo.try_resolve(key).unwrap());
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.bench_function("resolve_unchecked", |b| {
        b.iter_custom(|iters| {
            run_threaded_filled(
                |rodeo, keys| {
                    for key in keys {
                        unsafe { black_box(rodeo.resolve_unchecked(key)) };
                    }
                },
                NUM_THREADS,
                iters,
                FxBuildHasher::default(),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    rodeo_std,
    rodeo_ahash,
    rodeo_fxhash,
    rodeo_std_threaded,
    rodeo_ahash_threaded,
    rodeo_fxhash_threaded
);
criterion_main!(benches);
