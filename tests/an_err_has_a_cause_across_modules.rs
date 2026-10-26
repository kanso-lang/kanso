//! `.cause` reads the err an `annotate` wrapped. A rescue opens only a failure
//! another package raised (#2087), so a cause is read where the annotating
//! package is foreign: `config` annotates std/text's failure, and the entry
//! rescues config's. The other readers, and `.cause` on a failure with no
//! cause, are in `tests/golden/micro/an_err_has_readers.kso`.

use std::process::Command;

#[test]
fn a_cause_is_read_from_another_modules_annotation() {
    for engine in [&[][..], &["--interp"][..]] {
        let mut args = vec!["run", "tests/golden/cause"];
        args.extend_from_slice(engine);
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(&args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("kanso binary runs");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "config: to_bytes takes byte values (0-255)\n\
             to_bytes takes byte values (0-255)\n\
             <none>\n\
             config/read\n\
             text/to_bytes\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "engine {engine:?}");
    }
}
