use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn evaluates_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_djson"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start djson");

    child
        .stdin
        .take()
        .expect("djson stdin was not piped")
        .write_all(b"1 + 2")
        .expect("failed to write djson input");

    let output = child.wait_with_output().expect("failed to wait for djson");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"Int(\n    3,\n)\n");
}
