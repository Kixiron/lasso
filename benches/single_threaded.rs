use lasso::{Cord, Rodeo};

use core::hash::BuildHasher;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::collections::hash_map::RandomState;
use string_interner::{StringInterner, Sym};

// TODO: More inputs, benchmark all of Rodeo's functions & benchmark ThreadedRodeo, RodeoReader and RodeoResolver

static INPUT: &'static str = include_str!("input.txt");

lazy_static::lazy_static! {
    static ref INPUT_LINES: Vec<&'static str> =
        INPUT.split_whitespace().collect::<Vec<&str>>();
}

fn bench_lines() -> &'static [&'static str] {
    &INPUT_LINES
}

struct EmptySetup {
    lines: &'static [&'static str],
    build_hasher: RandomState,
}

impl EmptySetup {
    pub fn new() -> Self {
        let lines = bench_lines();

        EmptySetup {
            lines,
            build_hasher: RandomState::new(),
        }
    }

    pub fn empty_rodeo(&self) -> Rodeo<Cord, RandomState> {
        Rodeo::with_capacity_and_hasher(self.lines.len(), self.build_hasher.clone())
    }

    pub fn empty_interner(&self) -> StringInterner<Sym> {
        StringInterner::with_capacity_and_hasher(self.lines.len(), self.build_hasher.clone())
    }

    pub fn lines(&self) -> &'static [&'static str] {
        self.lines
    }
}

fn empty_setup() -> EmptySetup {
    EmptySetup::new()
}

struct FilledSetup<H>
where
    H: BuildHasher + Clone,
{
    lines: &'static [&'static str],
    rodeo: Rodeo<Cord, H>,
    interner: StringInterner<Sym>,
    keys: Vec<Cord>,
    symbols: Vec<Sym>,
}

impl FilledSetup<RandomState> {
    pub fn new() -> Self {
        let lines = bench_lines();
        let mut rodeo = Rodeo::with_capacity(lines.len());
        let keys = lines
            .into_iter()
            .map(|&line| rodeo.get_or_intern(line))
            .collect::<Vec<_>>();
        let mut interner = StringInterner::with_capacity(lines.len());
        let symbols = lines
            .into_iter()
            .map(|&line| interner.get_or_intern(line))
            .collect::<Vec<_>>();

        FilledSetup {
            lines,
            rodeo,
            interner,
            keys,
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

    pub fn filled_rodeo(&self) -> &Rodeo<Cord, H> {
        &self.rodeo
    }

    pub fn filled_rodeo_mut(&mut self) -> &mut Rodeo<Cord, H> {
        &mut self.rodeo
    }

    pub fn filled_interner(&self) -> &StringInterner<Sym> {
        &self.interner
    }

    pub fn filled_interner_mut(&mut self) -> &mut StringInterner<Sym> {
        &mut self.interner
    }

    pub fn keys(&self) -> &[Cord] {
        &self.keys
    }

    pub fn symbols(&self) -> &[Sym] {
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
            let mut rodeo = setup.empty_rodeo();
            for &line in setup.lines() {
                black_box(rodeo.get_or_intern(line));
            }
        })
    });

    let mut setup = filled_setup();
    group.bench_function("get_or_intern filled", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().get_or_intern(line));
            }
        })
    });

    let setup = empty_setup();
    group.bench_function("try_get_or_intern empty", |b| {
        b.iter(|| {
            let mut rodeo = setup.empty_rodeo();
            for &line in setup.lines() {
                black_box(rodeo.try_get_or_intern(line).unwrap());
            }
        })
    });

    let mut setup = filled_setup();
    group.bench_function("try_get_or_intern filled", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_rodeo_mut().try_get_or_intern(line).unwrap());
            }
        })
    });

    let setup = empty_setup();
    group.bench_function("get empty", |b| {
        b.iter(|| {
            let rodeo = setup.empty_rodeo();
            for &line in setup.lines() {
                black_box(rodeo.get(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("get filled", |b| {
        b.iter(|| {
            let rodeo = setup.filled_rodeo();
            for &line in setup.lines() {
                black_box(rodeo.get(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("resolve filled", |b| {
        b.iter(|| {
            let rodeo = setup.filled_rodeo();
            for key in setup.keys() {
                black_box(rodeo.resolve(key));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("try_resolve filled", |b| {
        b.iter(|| {
            let rodeo = setup.filled_rodeo();
            for key in setup.keys() {
                black_box(rodeo.try_resolve(key).unwrap());
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("resolve_unchecked filled", |b| {
        b.iter(|| {
            let rodeo = setup.filled_rodeo();
            for key in setup.keys() {
                unsafe {
                    black_box(rodeo.resolve_unchecked(key));
                }
            }
        })
    });

    let setup = empty_setup();
    group.bench_function("string-interner get_or_intern empty", |b| {
        b.iter(|| {
            let mut interner = setup.empty_interner();
            for &line in setup.lines() {
                black_box(interner.get_or_intern(line));
            }
        })
    });

    let mut setup = filled_setup();
    group.bench_function("string-interner get_or_intern filled", |b| {
        b.iter(|| {
            for &line in setup.lines() {
                black_box(setup.filled_interner_mut().get_or_intern(line));
            }
        })
    });

    let setup = empty_setup();
    group.bench_function("string-interner get empty", |b| {
        b.iter(|| {
            let interner = setup.empty_interner();
            for &line in setup.lines() {
                black_box(interner.get(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("string-interner get filled", |b| {
        b.iter(|| {
            let interner = setup.filled_interner();
            for &line in setup.lines() {
                black_box(interner.get(line));
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("string-interner resolve filled", |b| {
        b.iter(|| {
            let interner = setup.filled_interner();
            for key in setup.symbols() {
                black_box(interner.resolve(*key).unwrap());
            }
        })
    });

    let setup = filled_setup();
    group.bench_function("string-interner resolve_unchecked filled", |b| {
        b.iter(|| {
            let interner = setup.filled_interner();
            for key in setup.symbols() {
                unsafe {
                    black_box(interner.resolve_unchecked(*key));
                }
            }
        })
    });

    group.finish();
}

criterion_group!(benches, throughput);
criterion_main!(benches);
