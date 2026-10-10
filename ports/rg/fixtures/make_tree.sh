#!/bin/sh
# Builds the search tree the fixtures run against, at the directory named by
# $1. The tree is generated rather than checked in because it holds
# .gitignore files of its own: committed, they would make the enclosing
# repository drop the very files they are there to hide.
set -e
T=$1
rm -rf "$T"
mkdir -p "$T"
cd "$T"
mkdir -p .hidden target vendor src/util docs/nested scripts data

cat > .gitignore <<'EOF'
# build output and logs
target/
*.log
!keep.log
/secret.txt
docs/*.tmp
EOF
printf 'vendor/\n' > .ignore
printf 'TODO: hidden note\n' > .hidden/notes.txt
cat > README.md <<'EOF'
# Widget

A small widget library. See docs/guide.md for usage.

TODO: write the install section
FIXME: the example below is out of date

    widget --fast input.txt

Contact: maintainer@example.com
EOF
printf 'TODO: rotate the secret\n' > secret.txt
printf 'TODO keep this log\n' > keep.log
printf 'TODO debug log line\n' > debug.log
printf 'TODO built artifact\n' > target/out.txt
printf '/* TODO vendored */\nint vendored(void);\n' > vendor/lib.c
cat > src/main.rs <<'EOF'
use std::env;
use std::process;

mod util;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: widget <file>");
        process::exit(2);
    }
    // TODO: support more than one file
    let name = &args[1];
    match util::strings::load(name) {
        Ok(text) => println!("{}", text),
        Err(err) => {
            eprintln!("error: {}", err);
            process::exit(1);
        }
    }
}
EOF
cat > src/lib.rs <<'EOF'
pub mod util;

/// The widget itself.
pub struct Widget {
    pub name: String,
    pub size: usize,
}

impl Widget {
    pub fn new(name: &str) -> Widget {
        Widget { name: name.to_string(), size: 0 }
    }

    // FIXME: size should be validated
    pub fn resize(&mut self, size: usize) {
        self.size = size;
    }
}
EOF
cat > src/util/strings.rs <<'EOF'
use std::fs;
use std::io::Error;

pub fn load(path: &str) -> Result<String, Error> {
    fs::read_to_string(path)
}

pub fn shout(s: &str) -> String {
    s.to_uppercase()
}

// todo: lower-case todo should only match with -i
EOF
printf 'generated.rs\n' > src/util/.gitignore
printf '// TODO generated code, ignored by src/util/.gitignore\n' \
  > src/util/generated.rs
printf 'TODO: this secret.txt is not at the root\n' > src/secret.txt
cat > docs/guide.md <<'EOF'
# Guide

Install the widget, then run it:

    widget input.txt

The widget reads one file. TODO: document flags.
Errors go to standard error.
EOF
printf 'TODO draft that .gitignore hides\n' > docs/draft.tmp
printf 'TODO nested tmp is not hidden by docs/*.tmp\n' > docs/nested/old.tmp
printf '!*.log\n' > scripts/.ignore
printf 'TODO: a log .gitignore hides and scripts/.ignore lets back in\n' \
  > scripts/run.log
cat > scripts/build.sh <<'EOF'
#!/bin/sh
set -e
# TODO: cache the build
cargo build --release
EOF
cat > scripts/deploy.py <<'EOF'
import sys

def deploy(target):
    # FIXME: no rollback yet
    print("deploying to", target)

if __name__ == "__main__":
    deploy(sys.argv[1])
EOF
printf 'header TODO\n\001\002\000\003binary TODO tail\n' > data/blob.bin
printf 'caf\351 latin-1 bytes\nplain line\n' > data/latin1.txt
: > data/empty.txt
printf 'na\303\257ve caf\303\251 TODO\nCAF\303\211 in capitals\n' > docs/unicode.txt
printf 'first line\nlast line without newline TODO' > data/noeol.txt
printf '*.csv\n' > data/.rgignore
printf 'id,note\n1,TODO csv\n' > data/table.csv
cat > context.txt <<'EOF'
one
two
match alpha
three
four
five
match beta
six
match gamma
seven
eight
nine
ten
eleven
match delta
EOF
