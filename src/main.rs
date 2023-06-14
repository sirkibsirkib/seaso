mod lang;

use lang::*;
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
    let mut parse_result = parse::seaso::program(&source);
    let start_i1 = Instant::now();
    if let Ok((_input, program)) = &mut parse_result {
        preprocessing::deanonymize_variable_ids(program);
        let executable_result = program.executable();
        let start_i2 = Instant::now();
        if let Ok(executable_program) = &executable_result {
            use dynamics::Executable as _;
            let denotation = executable_program.denotation();
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
            use lang::search::DomainBadnessOrder;
            let domains = [DomainId("guy".into()), DomainId("none".into())];
            let dbo = DomainBadnessOrder(&domains);
            println!("search {:#?}", executable_program.search(dbo));
        }
        println!("undeclared domains: {:?}", program.undeclared_domains());
        println!("seal broken: {:?}", program.seal_break());
        println!("executable error {:#?}", executable_result.err());
    }
    println!("parse error {:?}", parse_result.err());

    Ok(())
}
