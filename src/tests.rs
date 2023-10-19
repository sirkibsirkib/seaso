use crate::cli::config::Config;
use std::ffi::OsStr;
use std::path::Path;

struct Null;
impl std::io::Write for Null {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

fn run_test(path: &Path) -> Result<(), ()> {
    let contents = std::fs::read_to_string(path).map_err(drop)?;
    let config = Config::no_flags();
    crate::cli::run::run_check(config, contents, &mut Null).map(drop).map_err(drop)
}

fn test_all(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if path.extension() == Some(&OsStr::new("seaso")) {
        let pass = run_test(path).is_ok();
        let sign = if pass { "pass" } else { "FAIL" };
        println!("{} {}", sign, path.display());
    }
    if let Ok(children) = std::fs::read_dir(path) {
        for child in children {
            test_all(child.unwrap().path())
        }
    }
}

#[test]
fn examples() {
    test_all("./example_programs")
}
