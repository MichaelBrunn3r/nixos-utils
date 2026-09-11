use std::{hint::black_box, rc::Rc};

use criterion::{Criterion, criterion_group, criterion_main};
use djson::{
    eval::{Scope, evaluate_ast, stdlib},
    parser::Parser,
};

fn evaluate(c: &mut Criterion) {
    let input = include_str!("./res/example.dj");
    let ast = Parser::new(input).parse_stmnts().expect("valid input");
    let root = stdlib::new();

    c.bench_function("evaluate", |benchmark| {
        benchmark.iter(|| {
            let mut scope = Scope::child(Rc::clone(&root));
            black_box(evaluate_ast(&ast, &mut scope))
        });
    });
}

criterion_group!(benches, evaluate);
criterion_main!(benches);
