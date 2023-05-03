mod ast;
mod dynamics;
mod parse;
mod statics;

fn remove_line_comments(mut s: String) -> String {
    let mut outside_comment = true;
    s.retain(|c| {
        if c == '#' {
            outside_comment = false;
        } else if c == '\n' {
            outside_comment = true;
        }
        outside_comment
    });
    s
}

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn main() -> Result<(), ()> {
    use std::time::Instant;
    let source = remove_line_comments(stdin_to_string().expect("bad stdin"));
    let start_i0 = Instant::now();
    let parse_result = parse::program(&source);
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
                "Denotation:\n- trues: {:?}\n- unknowns: {:?}\n- emitted: {:?}",
                &den.trues, &den.unknowns, &den.emitted,
            );
        }
        println!("seal broken: {:?}", program.seal_break());
        println!("undeclared domains: {:?}", program.undeclared_domains());
        println!("check result {:#?}", check_result.as_ref().map(drop));
    }
    println!("parse result {:?}", parse_result.as_ref().map(drop));
    Ok(())
}
