#!/usr/bin/env -S env -u CARGO_BUILD_TARGET -u CARGO_BUILD_PROFILE -u CARGO_UNSTABLE_BUILD_STD -u CARGO_UNSTABLE_BUILD_STD_FEATURES -u CARGO_UNSTABLE_PANIC_ABORT_TESTS cargo
---
[package]
edition = "2024"
---

//! Cargo runner for the PS2 target (see `.cargo/config.toml`).
//!
//! The shebang runs this through `cargo` (single-file package, `-Zscript` is on
//! in `.cargo/config.toml`) after unsetting the cross-compile variables that
//! `just ps2-cargo` exports - this program has to run on the *host*, and those
//! variables would otherwise hand it to the PS2 target.
//!
//! Boots an ELF in PCSX2 and mirrors the emulated EE console onto stdout, which
//! is the only way to see anything from a bare-metal binary here.
//!
//! Note that the EE console does *not* arrive on PCSX2's stdout. `-earlyconsolelog`
//! only affects PCSX2's own log verbosity, and the guest's `_print` output is
//! written to the emulog, so everything below reads `-logfile` and tails it.
//!
//! Two modes, picked by what is inside the ELF:
//!
//!   * test mode - the binary contains one of the harness' result markers
//!     (`tests/ps2.rs`). The log is watched for a marker and the exit status
//!     follows: pass -> 0, fail -> 1, nothing within `PS2_TEST_TIMEOUT` -> 1.
//!     This is what makes `just ps2-cargo test` report failures to cargo.
//!
//!   * run mode  - anything else (the game). Stream the console until the
//!     emulator exits or we are interrupted.
//!
//! Ctrl-C reaches the emulator too, since it shares the terminal's foreground
//! process group. Killing *just* this runner (SIGKILL, or `timeout`) leaves
//! PCSX2 running and its temp files behind.
//!
//! Environment:
//!   PCSX2             emulator to launch        (default: pcsx2-qt)
//!   PS2_TEST_TIMEOUT  test mode budget seconds  (default: 120)

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Printed by the harness once every test has run.
const PASSED: &str = "TESTS FINISHED - ALL PASSED";

/// Printed by the harness' panic handler.
const FAILED: &str = "TESTS FINISHED - FAILED";

fn main() {
    let elf = match env::args().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => {
            eprintln!("[pcsx2] usage: <path-to-elf>");
            std::process::exit(2);
        }
    };
    let elf = match elf.canonicalize() {
        Ok(elf) => elf,
        Err(err) => {
            eprintln!("[pcsx2] cannot open {}: {err}", elf.display());
            std::process::exit(2);
        }
    };

    let pcsx2 = env::var("PCSX2").unwrap_or_else(|_| "pcsx2-qt".into());
    let timeout = env::var("PS2_TEST_TIMEOUT")
        .ok()
        .and_then(|secs| secs.parse().ok())
        .unwrap_or(120);
    let timeout = Duration::from_secs(timeout);

    // The markers live in the binary's .rodata, so a byte search is enough to
    // tell a test ELF from the game.
    let test_mode = contains_marker(&elf, PASSED.as_bytes());

    let log = temp_path("console.log");
    let stderr = temp_path("stderr.log");

    println!("[pcsx2] booting {}", elf.display());
    if test_mode {
        println!("[pcsx2] test mode: watching for '{PASSED}' (timeout {timeout:?})");
    }

    // PCSX2's own complaints (missing BIOS, bad ELF, GPU init failures) go to
    // stderr and are only interesting when the run produces no verdict at all.
    let stderr_file = File::create(&stderr).ok();
    let mut vm = Command::new(&pcsx2)
        .args(["-batch", "-nogui", "-earlyconsolelog"])
        .arg("-logfile")
        .arg(&log)
        .arg("-elf")
        .arg(&elf)
        .stdout(Stdio::null())
        .stderr(
            stderr_file
                .map(Stdio::from)
                .unwrap_or_else(|| Stdio::null()),
        )
        .spawn()
        .unwrap_or_else(|err| {
            eprintln!("[pcsx2] failed to start {pcsx2}: {err}");
            std::process::exit(1);
        });

    // Lines from the guest console, as they are appended to the emulog.
    let lines = tail(log.clone(), vm.id());

    let outcome = if test_mode {
        verdict(&lines, timeout)
    } else {
        relay(&lines)
    };

    shutdown(&mut vm);
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_file(&stderr);

    match outcome {
        Outcome::Pass => println!("[pcsx2] tests passed"),
        Outcome::Finished => {}
        Outcome::Fail(reason) => {
            eprintln!("[pcsx2] {reason}");
            if let Some(diag) = last_log_lines(&stderr) {
                eprintln!("[pcsx2] ---- emulator stderr ----\n{diag}");
            }
            std::process::exit(1);
        }
    }
}

enum Outcome {
    /// Every test ran and the pass marker showed up.
    Pass,
    /// Run mode: the emulator closed on its own.
    Finished,
    /// Something failed, with the reason for the log.
    Fail(String),
}

/// True when `needle` appears anywhere in `path`.
fn contains_marker(path: &Path, needle: &[u8]) -> bool {
    use std::io::Read;

    let mut bytes = Vec::new();
    match File::open(path).and_then(|mut file| file.read_to_end(&mut bytes)) {
        Ok(_) => bytes.windows(needle.len()).any(|win| win == needle),
        Err(_) => false,
    }
}

fn temp_path(suffix: &str) -> PathBuf {
    env::temp_dir().join(format!("pcsx2-{}-{suffix}", std::process::id()))
}

/// Streams `log` line by line until the emulator dies, forwarding each line to
/// stdout and to the returned receiver.
///
/// The reader runs on its own thread because `read_line` blocks: the main thread
/// needs to be able to notice a timeout even while no output is arriving.
fn tail(log: PathBuf, pid: u32) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // The emulog is created a moment after the process starts.
        let mut reader = loop {
            if let Ok(file) = File::open(&log) {
                break BufReader::new(file);
            }
            if !process_alive(pid) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        };

        let mut line = String::new();
        let mut alive = true;

        while alive {
            line.clear();
            match reader.read_line(&mut line) {
                // Data: hand it on. EOF (Ok(0)): fall through and poll again.
                Ok(0) => {
                    alive = process_alive(pid);
                    if !alive {
                        break;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Ok(_) => {
                    let line = line.trim_end_matches(['\r', '\n']).to_string();
                    println!("{line}");
                    if tx.send(line).is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }

        // The emulator is gone: flush whatever it wrote on the way out, so the
        // last test line (or a panic message) still reaches the verdict.
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            println!("{line}");
            let _ = tx.send(line);
        }
    });

    rx
}

fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Reads verdict markers, giving up once the timeout expires.
fn verdict(lines: &Receiver<String>, timeout: Duration) -> Outcome {
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Outcome::Fail(format!("no verdict within {timeout:?}"));
        }

        match lines.recv_timeout(Duration::from_millis(100)) {
            Ok(line) if line.contains(PASSED) => return Outcome::Pass,
            Ok(line) if line.contains(FAILED) => return Outcome::Fail("tests failed".into()),
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Outcome::Fail("emulator exited before the tests reported a verdict".into());
            }
        }
    }
}

/// Run mode: mirror the console until the emulator goes away on its own.
fn relay(lines: &Receiver<String>) -> Outcome {
    while let Ok(_line) = lines.recv() {}
    Outcome::Finished
}

/// SIGTERM, then SIGKILL if the emulator is being stubborn about it.
fn shutdown(vm: &mut Child) {
    if !process_alive(vm.id()) {
        return;
    }

    let _ = Command::new("kill")
        .arg(vm.id().to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    for _ in 0..50 {
        if vm.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = vm.kill();
    let _ = vm.wait();
}

fn last_log_lines(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let tail: Vec<&str> = text.lines().rev().take(20).collect();
    if tail.is_empty() {
        None
    } else {
        Some(tail.into_iter().rev().collect::<Vec<_>>().join("\n"))
    }
}
