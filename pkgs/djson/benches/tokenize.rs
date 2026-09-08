use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use djson::lexer::Lexer;

fn tokenize(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");

    c.bench_function("tokenize", |benchmark| {
        benchmark.iter(|| black_box(Lexer::new(input).collect::<Vec<_>>()));
    });
}

criterion_group!(benches, tokenize);
criterion_main!(benches);
