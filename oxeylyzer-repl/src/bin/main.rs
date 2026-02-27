use oxeylyzer_repl::repl;

fn main() -> Result<(), repl::ReplError> {
    repl::Repl::run()
}
