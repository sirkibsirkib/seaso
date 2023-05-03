mod ast;
mod dynamics;
mod parse;
mod preprocessing;
mod statics;

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
    let source = preprocessing::line_comments_removed(stdin_to_string().expect("bad stdin"));
    let start_i0 = Instant::now();
    let parse_result = parse::program(&source)
        .map(|(s, program)| (s, preprocessing::variable_ids_deanonymized(program)));
    let start_i1 = Instant::now();
    if let Ok((_input, program)) = &parse_result {
        let check_result = program.check();
        let start_i2 = Instant::now();
        if let Ok(r2v2d) = &check_result {
            let denotation = program.denotation(&r2v2d);
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
