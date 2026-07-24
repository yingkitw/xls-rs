//! Integration tests for stdin/stdout piping ergonomics.

use std::process::Command;

fn xls_rs_exe() -> &'static str {
    env!("CARGO_BIN_EXE_xls-rs")
}

#[test]
fn test_pipe_stdin_to_stdout_sort() {
    let input = "name,score\nAlice,30\nBob,10\nCarol,20\n";

    let out = Command::new(xls_rs_exe())
        .args(["--quiet", "sort", "--column", "score", "--ascending"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let mut child = out;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "name,score");
    assert_eq!(lines[1], "Bob,10");
    assert_eq!(lines[2], "Carol,20");
    assert_eq!(lines[3], "Alice,30");
}

#[test]
fn test_pipe_chained_sort_then_head() {
    let input = "name,score\nAlice,30\nBob,10\nCarol,20\nDave,40\nEve,5\n";

    // Step 1: sort via stdin → stdout
    let sort_child = Command::new(xls_rs_exe())
        .args(["--quiet", "sort", "--column", "score", "--ascending"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let mut sort_child = sort_child;
    {
        let mut stdin = sort_child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }
    let sort_output = sort_child.wait_with_output().unwrap();
    assert!(sort_output.status.success());

    // Step 2: pipe sorted output into head -n 2
    let head_out = Command::new(xls_rs_exe())
        .args(["--quiet", "head", "-n", "2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut head_child = head_out;
    {
        let mut stdin = head_child.stdin.take().unwrap();
        stdin.write_all(&sort_output.stdout).unwrap();
    }
    let head_output = head_child.wait_with_output().unwrap();
    assert!(
        head_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&head_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&head_output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "name,score");
    assert_eq!(lines[1], "Eve,5");
}

#[test]
fn test_pipe_stdin_head_default_input() {
    let input = "a,b,c\n1,2,3\n4,5,6\n7,8,9\n";

    let out = Command::new(xls_rs_exe())
        .args(["--quiet", "head", "-n", "2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let mut child = out;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "a,b,c");
    assert_eq!(lines[1], "1,2,3");
}

#[test]
fn test_pipe_stdin_describe() {
    let input = "x,y\n1,10\n2,20\n3,30\n";

    let out = Command::new(xls_rs_exe())
        .args(["--quiet", "describe"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let mut child = out;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x"));
    assert!(stdout.contains("count"));
}

#[test]
fn test_pipe_stdin_select_to_stdout() {
    let input = "a,b,c\n1,2,3\n4,5,6\n";

    let out = Command::new(xls_rs_exe())
        .args(["--quiet", "select", "--columns", "a,c"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    let mut child = out;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "a,c");
    assert_eq!(lines[1], "1,3");
    assert_eq!(lines[2], "4,6");
}
