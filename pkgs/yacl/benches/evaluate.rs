use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use yacl::{eval::evaluate_ast, parser::Parser, stdlib};

fn evaluate(c: &mut Criterion) {
    let input = include_str!("./res/example.yacl");
    let ast = Parser::new(input).parse().expect("valid input");
    let root = stdlib::new();

    c.bench_function("evaluate", |benchmark| {
        benchmark.iter(|| black_box(evaluate_ast(&ast, &root)));
    });
}

criterion_group!(benches, evaluate);
criterion_main!(benches);
