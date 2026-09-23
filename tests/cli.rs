#![cfg(windows)]

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn listener_fixture() {
    let Ok(address) = std::env::var("PORT_GOBLIN_TEST_LISTENER") else {
        return;
    };
    let listener = TcpListener::bind((address.as_str(), 0)).unwrap();
    println!("GOBLIN_PORT={}", listener.local_addr().unwrap().port());
    std::io::stdout().flush().unwrap();
    // Bound lifetime even if the parent test crashes.
    std::thread::sleep(Duration::from_secs(30));
    drop(listener);
}

fn exercise_listener(address: &str) {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "listener_fixture", "--nocapture"])
        .env("PORT_GOBLIN_TEST_LISTENER", address)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut fixture = Fixture(child);
    let mut output = BufReader::new(fixture.0.stdout.take().unwrap());
    let port = loop {
        let mut line = String::new();
        assert!(
            output.read_line(&mut line).unwrap() > 0,
            "listener failed to start"
        );
        if let Some((_, port)) = line.split_once("GOBLIN_PORT=") {
            break port.trim().to_owned();
        }
    };
    let binary = env!("CARGO_BIN_EXE_port-goblin");
    let report = Command::new(binary).arg(&port).output().unwrap();
    assert!(report.status.success(), "{:?}", report);
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(text.contains("OCCUPIED"), "{text}");
    assert!(text.contains(&format!("PID {}", fixture.0.id())), "{text}");
    let killed = Command::new(binary)
        .args([&port, "--kill-mode"])
        .output()
        .unwrap();
    assert!(killed.status.success(), "{:?}", killed);
    assert!(String::from_utf8_lossy(&killed.stdout).contains("FREE"));
    fixture.0.wait().unwrap();
    let free = Command::new(binary).arg(&port).output().unwrap();
    assert!(free.status.success());
    assert!(String::from_utf8_lossy(&free.stdout).contains("FREE"));
}

#[test]
fn inspects_and_terminates_disposable_ipv4_listener() {
    exercise_listener("127.0.0.1");
}

#[test]
fn inspects_and_terminates_disposable_ipv6_listener() {
    if TcpListener::bind(("::1", 0)).is_ok() {
        exercise_listener("::1");
    }
}

#[test]
fn help_and_invalid_port_exit_codes() {
    let binary = env!("CARGO_BIN_EXE_port-goblin");
    assert!(
        Command::new(binary)
            .arg("--help")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        Command::new(binary)
            .arg("0")
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}
