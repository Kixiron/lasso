use lasso::{Cord, Rodeo};

use core::hash::{BuildHasher, BuildHasherDefault, Hasher};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::collections::hash_map::RandomState;

static INPUT: &'static str = include_str!("input.txt");

lazy_static::lazy_static! {
    static ref INPUT_LINES: Vec<&'static str> =
        INPUT.split_whitespace().collect::<Vec<&str>>();
}

fn bench_lines() -> &'static [&'static str] {
    &INPUT_LINES
}

struct EmptySetup<H> {
    lines: &'static [&'static str],
    build_hasher: H,
}

impl EmptySetup<RandomState> {
    pub fn new() -> Self {
        let lines = bench_lines();
        EmptySetup {
            lines,
            build_hasher: RandomState::new(),
        }
    }
}

impl<H> EmptySetup<H>
where
    H: BuildHasher + Clone,
{
    pub fn new_with_hasher() -> Self {
        let lines = bench_lines();
        let build_hasher = BuildHasherDefault::<S>::default();

        EmptySetup {
            lines,
            build_hasher,
        }
    }

    pub fn empty_interner(&self) -> Rodeo<Cord, H> {
        Rodeo::with_capacity_and_hasher(self.lines.len(), self.build_hasher.clone())
    }

    pub fn lines(&self) -> &'static [&'static str] {
        self.lines
    }
}

fn empty_setup() -> EmptySetup<RandomState> {
    EmptySetup::new()
}

struct FilledSetup<H>
where
    H: BuildHasher + Clone,
{
    lines: &'static [&'static str],
    interner: Rodeo<Cord, H>,
    symbols: Vec<Cord>,
}

impl FilledSetup<RandomState> {
    pub fn new() -> Self {
        let lines = bench_lines();
        let mut interner = Rodeo::with_capacity(lines.len());
        let symbols = lines
            .into_iter()
            .map(|&line| interner.get_or_intern(line))
            .collect::<Vec<_>>();

        FilledSetup {
            lines,
            interner,
            symbols,
        }
    }
}

impl<S> FilledSetup<BuildHasherDefault<S>>
where
    S: Hasher + Clone + Default,
{
    pub fn new_with_hasher() -> Self {
        let lines = bench_lines();
        let build_hasher = BuildHasherDefault::<S>::default();
        let mut interner = Rodeo::with_capacity_and_hasher(lines.len(), build_hasher);
        let symbols = lines
            .into_iter()
            .map(|&line| interner.get_or_intern(line))
            .collect::<Vec<_>>();

        FilledSetup {
            lines,
            interner,
            symbols,
        }
    }
}

impl<H> FilledSetup<H>
where
    H: BuildHasher + Clone,
{
    pub fn lines(&self) -> &'static [&'static str] {
        self.lines
    }

    pub fn filled_interner(&self) -> &Rodeo<Cord, H> {
        &self.interner
    }

    pub fn filled_interner_mut(&mut self) -> &mut Rodeo<Cord, H> {
        &mut self.interner
    }

    pub fn keys(&self) -> &[Cord] {
        &self.symbols
    }
}

fn filled_setup() -> FilledSetup<RandomState> {
    FilledSetup::new()
}

fn throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("single-threaded throughput");

    group.throughput(Throughput::Bytes(INPUT.len() as u64));

    let setup = empty_setup();
    group.bench_function("get_or_intern empty", |b| {
        b.iter(|| {
            let mut interner = setup.empty_interner();
            for &line in setup.lines() {
                black_box(interner.get_or_intern(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("get_or_intern filled", |b| {
        for &line in setup.lines() {
            black_box(setup.filled_interner_mut().get_or_intern(line));
        }
    });

    let setup = empty_setup();
    group.bench_function("try_get_or_intern empty", |b| {
        b.iter(|| {
            let mut interner = setup.empty_interner();
            for &line in setup.lines() {
                black_box(interner.try_get_or_intern(line).unwrap());
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("try_get_or_intern filled", |b| {
        for &line in setup.lines() {
            black_box(setup.filled_interner_mut().try_get_or_intern(line).unwrap());
        }
    });

    let setup = empty_setup();
    group.bench_function("get empty", |b| {
        b.iter(|| {
            let mut interner = setup.empty_interner();
            for &line in setup.lines() {
                black_box(interner.get(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("get filled", |b| {
        for &line in setup.lines() {
            black_box(setup.filled_interner().get(line));
        }
    });

    let setup = filled_setup();
    group.bench_function("resolve filled", |b| {
        for key in setup.keys() {
            black_box(setup.filled_interner().resolve(key));
        }
    });

    let setup = filled_setup();
    group.bench_function("try_resolve filled", |b| {
        for key in setup.keys() {
            black_box(setup.filled_interner().try_resolve(key));
        }
    });

    let setup = filled_setup();
    group.bench_function("resolve_unchecked filled", |b| {
        for key in setup.keys() {
            unsafe {
                black_box(setup.filled_interner().resolve_unchecked(key));
            }
        }
    });

    group.finish();
}

criterion_group!(benches, throughput);
criterion_main!(benches);
