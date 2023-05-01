use std::io::Read;
use std::time::Instant;
mod ast;
mod dynamics;
mod parse;
mod statics;

fn stdin_to_string() -> Result<String, std::io::Error> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    stdin.lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn main() -> Result<(), ()> {
    let source = stdin_to_string().expect("bad stdin");
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
                "times taken:\n  parse {:?}\n  check {:?}\n  denote {:?}",
                start_i1 - start_i0,
                start_i2 - start_i1,
                start_i3 - start_i2
            );
            println!("Denotation:\n  pos: {:?}\n  unk: {:?}", &den.pos, &den.unk);
        }
        println!("seal broken: {:?}", program.seal_break());
        println!("check result {:?}", check_result.as_ref().map(drop));
    }
    println!("parse result {:?}", parse_result.as_ref().map(drop));
    Ok(())
}
