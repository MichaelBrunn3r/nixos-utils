use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use djson::parser::Parser;

fn parse(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");

    c.bench_function("parse", |benchmark| {
        benchmark.iter(|| black_box(Parser::new(input).parse()));
    });
}

criterion_group!(benches, parse);
criterion_main!(benches);
