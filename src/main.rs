mod ast;
mod dynamics;
mod parse;
mod preprocessing;
// mod print;
mod statics;

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn main() -> Result<(), ()> {
    use std::time::Instant;
    let source = preprocessing::line_comments_removed(stdin_to_string().expect("bad stdin"));
    let start_i0 = Instant::now();
    let parse_result = parse::program(&source)
        .map(|(s, program)| (s, preprocessing::variable_ids_deanonymized(program)));
    let start_i1 = Instant::now();
    if let Ok((_input, program)) = &parse_result {
        let check_result = program.check();
        let start_i2 = Instant::now();
        if let Ok(r2v2d) = &check_result {
            let den = program.denotation(&r2v2d);
            let start_i3 = Instant::now();
            println!(
                "times taken:\n- parse {:?}\n- check {:?}\n- denote {:?}",
                start_i1 - start_i0,
                start_i2 - start_i1,
                start_i3 - start_i2
            );
            println!(
                "Denotation:\n- trues: {:?}\n- unknowns: {:?}\n- emissions: {:?}",
                &den.trues, &den.unknowns, &den.emissions,
            );
        }
        // println!("program {}", program.printed(true));
        println!("seal broken: {:?}", program.seal_break());
        println!("undeclared domains: {:?}", program.undeclared_domains());
        println!("check result {:#?}", check_result.as_ref().map(drop));
    }
    println!("parse result {:?}", parse_result.as_ref().map(drop));
    Ok(())
}
