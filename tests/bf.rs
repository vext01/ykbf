use std::{
    io::Write,
    process::{exit, Command, Stdio},
};

use tempfile::NamedTempFile;

fn test_stdout(prg: &str, stdout_str: &str) {
    test_stdin(prg, "", stdout_str);
}

fn test_stdin(prg: &str, stdin_str: &str, stdout_str: &str) {
    let tf = NamedTempFile::new().unwrap();
    tf.as_file().write_all(prg.as_bytes()).unwrap();
    let mut c = Command::new(env!("CARGO"))
        .args(&["run", tf.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin
        .as_mut()
        .unwrap()
        .write_all(stdin_str.as_bytes())
        .unwrap();
    let w = c.wait_with_output().unwrap();
    if !w.status.success() {
        println!("{}", String::from_utf8_lossy(&w.stdout));
        eprintln!("{}", String::from_utf8_lossy(&w.stderr));
        exit(1);
    }
    assert_eq!(String::from_utf8_lossy(&w.stdout), stdout_str);
}

#[test]
fn test_hello_world() {
    test_stdout("++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>.", "Hello World!\n");
}

#[test]
fn test_obscure_problems() {
    // From http://www.hevanet.com/cristofd/brainfuck/tests.b
    test_stdout(
        "[]++++++++++[>>+>+>++++++[<<+<+++>>>-]<<<<-]\"A*$\";?@![#>>+<<]>[>>]<<<<[>++<[-]]>.>.",
        "H\n",
    );
}

#[test]
fn test_echo() {
    test_stdin(",[.[-],]", "ABCdef", "ABCdef");
}
