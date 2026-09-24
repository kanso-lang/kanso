//! Every block in an emitted function is reached from its entry.
//!
//! The emitter opens a dispatcher's failure path before it knows whether any
//! parameter check will branch to it, and when every check is proved away
//! nothing does. Those blocks were 13% of the decoder's emitted lines, and
//! clang parsed each one only to delete it in its first pass.
//!
//! The program here has a dispatcher over two integers whose checks the
//! emitter proves away, and it maps over a list, which reaches std/list. Every
//! block left in its module must be reachable from its function's entry, and
//! the binary must print what the interpreter prints.
//!
//! Watched red with the pass left out: 22 blocks nothing reached, the first
//! of them `d_list/fold_3`'s `fail2`.

use std::collections::{HashMap, HashSet};
use std::process::Command;

const MAIN: &str = "import \"./counted\"\n\nplay\n";

const LIBRARY: &str = "import \"std/list\"

pub play = print \"sum {climbed 1 20 0} {list/sum (list/map [1 2 3] doubled)}\"

fn doubled x
  x * 2

fn climbed i stop acc
  reached i stop acc (i > stop)

fn reached _ _ acc true
  acc

fn reached i stop acc false
  climbed (i + 1) stop (acc + i)
";

/// The labels of each defined function that its entry cannot reach.
fn unreached(module: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut name = String::new();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut inside = false;
    for line in module.lines() {
        if line.starts_with("define ") {
            name = line.to_string();
            blocks = vec![(String::new(), Vec::new())];
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line == "}" {
            let at: HashMap<&str, usize> =
                blocks.iter().enumerate().map(|(i, (l, _))| (l.as_str(), i)).collect();
            let mut seen = HashSet::from([0usize]);
            let mut work = vec![0usize];
            while let Some(b) = work.pop() {
                for text in &blocks[b].1 {
                    for part in text.split("label %").skip(1) {
                        let end = part
                            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                            .unwrap_or(part.len());
                        if let Some(&next) = at.get(&part[..end]) {
                            if seen.insert(next) {
                                work.push(next);
                            }
                        }
                    }
                }
            }
            for (i, (label, _)) in blocks.iter().enumerate() {
                if !seen.contains(&i) {
                    found.push(format!("{label} in {name}"));
                }
            }
            inside = false;
            continue;
        }
        match line.strip_suffix(':') {
            Some(label) if !line.starts_with(' ') => {
                if blocks.len() == 1 && blocks[0].1.is_empty() {
                    blocks[0].0 = label.to_string();
                } else {
                    blocks.push((label.to_string(), Vec::new()));
                }
            }
            _ => blocks.last_mut().expect("a block").1.push(line.to_string()),
        }
    }
    found
}

#[test]
fn every_block_is_reached_from_its_entry() {
    let dir = std::env::temp_dir().join(format!("kanso_unreached_block_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::create_dir_all(dir.join("counted")).expect("a module directory");
    std::fs::write(dir.join("counted/counted.kso"), LIBRARY).expect("the library writes");
    std::fs::write(dir.join("main.kso"), MAIN).expect("the program writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg("main.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let native = Command::new(dir.join("main")).output().expect("the binary runs");
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("main.kso")
        .arg("--interp")
        .current_dir(&dir)
        .output()
        .expect("the interpreter runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(native.stdout, oracle.stdout, "the engines disagree");
    assert_eq!(String::from_utf8_lossy(&native.stdout), "sum 210 12\n");
    let left = unreached(&module);
    assert!(left.is_empty(), "{} blocks nothing reaches: {:?}", left.len(), left);
}
