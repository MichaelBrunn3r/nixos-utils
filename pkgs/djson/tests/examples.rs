use std::{
    fs,
    path::{Path, PathBuf},
};

use djson::{
    eval::{Scope, evaluate_ast, stdlib},
    parser::Parser,
};

const EXAMPLE_EXTENSIONS: [&str; 2] = ["dj", "json"];

#[test]
fn examples_evaluate_without_errors() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut paths: Vec<PathBuf> = fs::read_dir(&examples)
        .expect("failed to read the examples directory")
        .map(|entry| entry.expect("failed to read an example entry").path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| EXAMPLE_EXTENSIONS.contains(&extension))
        })
        .collect();
    paths.sort();

    assert!(
        !paths.is_empty(),
        "no example files found in {}",
        examples.display()
    );

    for path in paths {
        let input = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
        let ast = Parser::new(&input)
            .parse_stmnts()
            .unwrap_or_else(|error| panic!("failed to parse {path:?}: {error:?}"));
        let mut scope = Scope::child(stdlib::prelude());
        evaluate_ast(&ast, &mut scope)
            .unwrap_or_else(|error| panic!("failed to evaluate {path:?}: {error:?}"));
    }
}
