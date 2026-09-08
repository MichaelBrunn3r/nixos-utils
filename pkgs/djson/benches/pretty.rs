use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use djson::parser::Parser;

fn pretty(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");
    let ast = Parser::new(input).parse().expect("valid input");

    c.bench_function("pretty", |benchmark| {
        benchmark.iter(|| black_box(ast.pretty_string()));
    });
}

criterion_group!(benches, pretty);
criterion_main!(benches);
