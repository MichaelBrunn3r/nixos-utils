use std::{hint::black_box, rc::Rc};

use criterion::{Criterion, criterion_group, criterion_main};
use djson::{
    eval::{Scope, evaluate_ast, stdlib},
    parser::Parser,
};

fn pipeline(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");
    let root = stdlib::new();

    c.bench_function("parse_and_evaluate", |benchmark| {
        benchmark.iter(|| {
            let ast = Parser::new(input).parse().expect("valid input");
            let mut scope = Scope::child(Rc::clone(&root));
            black_box(evaluate_ast(&ast, &mut scope))
        });
    });
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
