//! A connection whose traffic goes through TLS from a point the program picks.
//!
//! Postgres asks for TLS inside a plain connection: the client sends an
//! SSLRequest, reads one byte, and only then starts the handshake on the same
//! socket. So the primitive is `net/secured c host`, which upgrades a
//! connection already open, and a dial straight into TLS is a dial followed by
//! it (#2178). Native code uses OpenSSL and the interpreter rustls, and both
//! check the server against the machine's trusted roots, which `SSL_CERT_FILE`
//! points at a test authority here.
//!
//! This harness plays the server with rustls and a certificate for `localhost`
//! signed by that authority (`tests/fixtures/tls`). Both engines that have
//! sockets run the same program and must print the same text. A server the
//! program cannot verify fails with a reason naming the host, worded alike on
//! both.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tls").join(name)
}

fn program(port: u16, host: &str) -> String {
    format!(
        "import \"std/net\"\n\n\
         fn spoken c\n  \
         net/write c \"hello\"\n    \
         .> (_ -> net/read c)\n    \
         .> (t -> print \"heard {{t}}\")\n    \
         .> (_ -> net/close_conn c)\n\n\
         net/dial \"localhost\" {port} .> (c -> net/secured c \"{host}\") .> spoken\n"
    )
}

/// Runs `source` on one engine from a directory of its own, trusting the roots
/// in `roots`, and answers what it printed and what it said on stderr.
fn ran(tag: &str, source: &str, interp: bool, roots: &std::path::Path) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kanso-tls-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("secure.kso"), source).expect("the program writes");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("play").arg("secure.kso").current_dir(&dir).env("SSL_CERT_FILE", roots);
    cmd.env_remove("SSL_CERT_DIR");
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

/// Listens, completes one TLS handshake as `localhost`, answers "world" to
/// what the program says, and hands back what it heard. A handshake the
/// program refuses ends the thread with nothing heard.
fn serve_once(goodbye: bool) -> (u16, std::thread::JoinHandle<Vec<u8>>) {
    let certs: Vec<CertificateDer<'static>> =
        CertificateDer::pem_file_iter(fixture("localhost.pem"))
            .expect("the certificate reads")
            .map(|c| c.expect("a certificate"))
            .collect();
    let key = PrivateKeyDer::from_pem_file(fixture("localhost.key")).expect("the key reads");
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the default versions")
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .expect("the server's certificate");
    let config = Arc::new(config);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a port to listen on");
    let port = listener.local_addr().expect("the port it was given").port();
    let served = std::thread::spawn(move || {
        // A program refused before it dials never connects, and the test
        // should report what it said rather than wait here for good.
        listener.set_nonblocking(true).expect("a listener that can give up");
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut tcp = loop {
            match listener.accept() {
                Ok((tcp, _)) => break tcp,
                Err(_) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(_) => return Vec::new(),
            }
        };
        tcp.set_nonblocking(false).expect("a blocking connection");
        tcp.set_read_timeout(Some(Duration::from_secs(10))).expect("a bounded read");
        let mut tls = rustls::ServerConnection::new(config).expect("a server session");
        let mut stream = rustls::Stream::new(&mut tls, &mut tcp);
        let mut heard = [0u8; 5];
        if stream.read_exact(&mut heard).is_err() {
            return Vec::new();
        }
        stream.write_all(b"world").expect("the answer goes out");
        stream.flush().expect("the answer is sent");
        if goodbye {
            tls.send_close_notify();
            let _ = tls.complete_io(&mut tcp);
        }
        heard.to_vec()
    });
    (port, served)
}

#[test]
fn a_program_secures_a_connection_and_talks_through_it_on_both_engines() {
    for (tag, interp) in [("native", false), ("interp", true)] {
        let (port, served) = serve_once(true);
        let (out, err) = ran(tag, &program(port, "localhost"), interp, &fixture("ca.pem"));
        let heard = served.join().expect("the server thread finishes");
        assert_eq!(out, "heard world\n", "{tag}: {err}");
        assert_eq!(heard, b"hello", "{tag}: the server heard something else");
    }
}

#[test]
fn a_server_the_program_cannot_verify_is_refused_alike_on_both_engines() {
    // The server's certificate is for localhost and signed by the test
    // authority. Trusting only the server's own certificate, which is no
    // authority, leaves the chain unverified; naming another host leaves the
    // name unmatched.
    let cases = [
        ("untrusted", "localhost", fixture("localhost.pem")),
        ("misnamed", "example.com", fixture("ca.pem")),
    ];
    for (case, host, roots) in cases {
        let want = format!(
            "error[endpoint]: unhandled err reached the executor: \"cannot secure the connection to {host}\"\n"
        );
        for (engine, interp) in [("native", false), ("interp", true)] {
            let (port, served) = serve_once(true);
            let tag = format!("{case}-{engine}");
            let (out, err) = ran(&tag, &program(port, host), interp, &roots);
            let _ = served.join();
            assert_eq!(out, "", "{tag}");
            assert_eq!(err, want, "{tag}");
        }
    }
}

/// A server that closes without TLS's close_notify, as plenty do, ends the
/// conversation the way a plain socket's close does: the next read is empty,
/// on both engines.
#[test]
fn a_close_without_goodbye_reads_as_the_end_on_both_engines() {
    for (tag, interp) in [("abrupt-native", false), ("abrupt-interp", true)] {
        let (port, served) = serve_once(false);
        let source = format!(
            "import \"std/net\"\n\n\
             fn spoken c\n  \
             net/write c \"hello\"\n    \
             .> (_ -> net/read c)\n    \
             .> (t -> print \"heard {{t}}\")\n    \
             .> (_ -> net/read c)\n    \
             .> (t -> print \"then {{length t}} bytes\")\n\n\
             net/dial \"localhost\" {port} .> (c -> net/secured c \"localhost\") .> spoken\n"
        );
        let (out, err) = ran(tag, &source, interp, &fixture("ca.pem"));
        let _ = served.join();
        assert_eq!(out, "heard world\nthen 0 bytes\n", "{tag}: {err}");
    }
}
