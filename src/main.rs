pub mod cli;
pub mod lang;

use cli::{
    config::Config,
    run::{run_check, stdin_to_string},
};

use lang::*;

fn main() {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let config = Config::from_sys_args();
    let source = stdin_to_string().expect("bad stdin");
    if let Err(e) = run_check(config, source, &mut stdout) {
        let _ = writeln!(&mut stdout, "{}", e);
    }
}
