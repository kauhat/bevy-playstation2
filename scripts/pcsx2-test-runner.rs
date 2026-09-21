#!/usr/bin/env cargo

use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("[Runner Error] Usage: ps2-test-runner <path-to-elf>");
        std::process::exit(1);
    }

    let elf_path = &args[1];
    println!("[Runner] Launching PCSX2 with: {}", elf_path);

    // Spawn PCSX2
    let mut child = Command::new("pcsx2-qt")
        .arg("-batch")
        .arg("-nogui")
        .arg("-earlyconsolelog")
        .arg("-elf")
        .arg(elf_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[Runner Error] Failed to start pcsx2-qt");

    let stdout = child.stdout.take().expect("Failed to open stdout");
    let reader = BufReader::new(stdout);

    let start_time = Instant::now();
    let mut test_passed = false;
    let mut finished_found = false;

    // Read stdout line by line
    for line in reader.lines() {
        // Enforce timeout
        if start_time.elapsed() > TIMEOUT {
            eprintln!("\n[Runner Error] Test timed out after {:?}", TIMEOUT);
            let _ = child.kill();
            std::process::exit(1);
        }

        match line {
            Ok(line_str) => {
                println!("{}", line_str);

                if line_str.contains("TESTS FINISHED - ALL PASSED") {
                    test_passed = true;
                    finished_found = true;
                    let _ = child.kill();
                    break;
                } else if line_str.contains("TESTS FINISHED - FAILED") {
                    finished_found = true;
                    let _ = child.kill();
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // Wait for the child process to fully exit
    let exit_status = child.wait().ok();

    if !finished_found {
        eprintln!("\n[Runner Error] PCSX2 exited or crashed before tests completed.");
        if let Some(status) = exit_status {
            eprintln!("[Runner Error] Exit status: {}", status);
        }
        std::process::exit(1);
    }

    if test_passed {
        println!("\n[Runner] Tests Completed Successfully.");
        std::process::exit(0);
    } else {
        eprintln!("\n[Runner] Tests Failed.");
        std::process::exit(1);
    }
}