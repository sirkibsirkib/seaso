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

fn zop() {
    let x = "
    module a:b   { seal y. }
    module b:c   { defn x(y). }
    module c:a,b { rule Y :- x(Y). }
    ";
    let (_, mo) = parse::modules(x).expect("WAHEY");
    println!("{:?}", mo);

    let module_system = modules::ModuleSystem::new(mo.iter()).expect("whaey");

    let mut sniffer = BreakSniffer::<&ModuleName>::default();

    let executable_result = ExecutableProgram::new(&module_system.map, &mut sniffer);

    let maybe_break = sniffer.find_break(&module_system);

    println!(
        "{:#?}\n{:#?}\n{:#?}\nbreak {:#?}",
        module_system, executable_result, sniffer, maybe_break
    );
    println!("//////////////////");
}

fn main() -> Result<(), ()> {
    zop();
    let mut source = stdin_to_string().expect("bad stdin");
    preprocessing::remove_line_comments(&mut source);
    let start_i0 = Instant::now();
    let mut statements_result = nom::combinator::all_consuming(parse::statements)(&source);
    let start_i1 = Instant::now();
    if let Ok((_input, statements)) = &mut statements_result {
        preprocessing::deanonymize_variable_ids(statements);

        let mut sniffer = BreakSniffer::<usize>::default();
        let structure = statements.0.as_slice();

        println!("structure {:?}", structure);
        let executable_result = ExecutableProgram::new(structure, &mut sniffer);
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

            let seal_break = sniffer.find_break(&());
            println!("seal broken: {:?}", seal_break);

            use search::DomainBadnessOrder;
            let domains = [DomainId("guy".into()), DomainId("none".into())];
            let dbo = DomainBadnessOrder(&domains);
            println!("search {:#?}", executable_program.search(dbo));
        }
        println!("undeclared domains: {:?}", statements.undeclared_domains());
        println!("executable error {:#?}", executable_result.err());
    }
    println!("statements error {:#?}", statements_result.err());

    Ok(())
}
