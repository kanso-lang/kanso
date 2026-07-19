pub struct ScriptedExecutor {
    pub transcript: Vec<String>,
    pub script_args: Vec<String>,
    pub script_stdin: String,
    pub files: std::collections::HashMap<String, String>,
}

impl Executor for ScriptedExecutor {
    fn print(&mut self, text: &str) {
        self.transcript.push(format!("print {text:?}"));
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        self.transcript.push(format!("read_file {path:?}"));
        self.files.get(path).cloned().ok_or_else(|| format!("cannot read {path}"))
    }
    // ...
}
