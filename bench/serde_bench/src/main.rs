//! The measuring stick for kanso's JSON gauntlet: serde_json, the parser a
//! Rust team would actually deploy. Mirrors bench/main.go's shape — decode
//! bench/large.json into an untyped value and time it — plus a repeated-run
//! variant so the number is stable rather than a single cold sample.
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "bench/large.json".to_string());
    let data = std::fs::read(&path).expect("read large.json");

    // single decode, matching main.go exactly
    let start = Instant::now();
    let v: serde_json::Value = serde_json::from_slice(&data).expect("parse");
    let single = start.elapsed();
    let top = v.as_array().map(|a| a.len()).unwrap_or(0);
    println!("serde decoded {top} top-level values in {single:?}");

    // 150 decodes, matching the kanso jsonbench harness; report the mean
    let runs = 150;
    let start = Instant::now();
    for _ in 0..runs {
        let _: serde_json::Value = serde_json::from_slice(&data).expect("parse");
    }
    let per = start.elapsed() / runs;
    println!("serde mean over {runs} decodes: {per:?}");
}
