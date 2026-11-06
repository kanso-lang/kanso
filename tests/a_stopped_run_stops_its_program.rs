//! `kanso run` builds a native binary and runs it as a child. Stopping the
//! driver with SIGTERM, as `kill` and process supervisors do, ended the driver
//! alone: the program went on running, reparented, and a server kept its port.
//! senro's specs start an app with `kanso run`, stop it with `kill`, and start
//! the next one on the same port, so the next run's requests reached the old
//! server. The driver now hands SIGTERM and SIGHUP to the program and ends the
//! way it does.
#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SIGTERM: i32 = 15;

/// Marks that it has started, then writes `finished` three seconds later,
/// which it does only if nothing stopped it.
const PROGRAM: &str = concat!(
    "import \"std/os\"\nimport \"std/time\"\n\n",
    "os/write_file \"started\" \"yes\"\n",
    "  .> (_ -> time/sleep 3000)\n",
    "  .> (_ -> os/write_file \"finished\" \"still running\")\n",
);

fn appears(path: &Path, within: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < within {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    path.exists()
}

#[test]
fn a_terminated_run_ends_its_native_program() {
    let work = std::env::temp_dir().join(format!("kanso-stopped-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("temp work dir");
    let program = work.join("sleeper.kso");
    std::fs::write(&program, PROGRAM).expect("program writes");

    let driver = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&program)
        .current_dir(&work)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso runs");
    assert!(appears(&work.join("started"), Duration::from_secs(120)), "the program started");
    let killed =
        Command::new("kill").args(["-TERM", &driver.id().to_string()]).status().expect("kill runs");
    assert!(killed.success(), "kill sent SIGTERM");
    let output = driver.wait_with_output().expect("the driver ends");

    assert!(
        !appears(&work.join("finished"), Duration::from_secs(5)),
        "the program was still running after its driver was terminated"
    );
    assert_eq!(
        output.status.signal(),
        Some(SIGTERM),
        "the driver ends by SIGTERM, as the program did: {:?}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "", "the driver says nothing");
    let _ = std::fs::remove_dir_all(&work);
}
