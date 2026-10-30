pub trait Executor {
    fn print(&mut self, text: &str);
    fn args(&mut self) -> Vec<String>;
    fn stdin(&mut self) -> Result<String, String>;
    fn read_file(&mut self, path: &str) -> Result<String, String>;
    fn write_file(&mut self, path: &str, content: &str) -> Result<(), String>;
}

impl Executor for RealExecutor {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }

    fn read_file(&mut self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
    }
    // ...
}
