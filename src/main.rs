mod ast;
mod dynamics;
mod parse;
mod preprocessing;
mod search;
mod statics;
mod util;

use std::time::{Duration, Instant};

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

#[allow(dead_code)]
#[derive(Debug)]
struct TimesTaken {
    parse: Duration,
    check: Duration,
    denote: Duration,
}

fn main() -> Result<(), ()> {
    let mut source = stdin_to_string().expect("bad stdin");
    preprocessing::remove_line_comments(&mut source);
    let start_i0 = Instant::now();
    let mut parse_result = parse::program(&source);
    let start_i1 = Instant::now();
    if let Ok((_input, program)) = &mut parse_result {
        preprocessing::deanonymize_variable_ids(program);
        let check_result = program.check();
        let start_i2 = Instant::now();
        if let Ok(checked) = &check_result {
            let denotation = checked.denotation();
            let start_i3 = Instant::now();
            println!(
                "{:#?}",
                TimesTaken {
                    parse: start_i1 - start_i0,
                    check: start_i2 - start_i1,
                    denote: start_i3 - start_i2
                }
            );
            println!("{:#?}", &denotation);
        }
        println!("undeclared domains: {:?}", program.undeclared_domains());
        println!("seal broken: {:?}", program.seal_break());
        println!("check error {:#?}", check_result.err());
    }
    println!("parse error {:?}", parse_result.err());
    Ok(())
}
