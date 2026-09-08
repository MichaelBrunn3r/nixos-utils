use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use djson::{eval::evaluate_ast, parser::Parser, stdlib};

fn pipeline(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");
    let root = stdlib::new();

    c.bench_function("parse_and_evaluate", |benchmark| {
        benchmark.iter(|| {
            let ast = Parser::new(input).parse().expect("valid input");
            black_box(evaluate_ast(&ast, &root))
        });
    });
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
