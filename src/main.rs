mod ast;
// mod data_structures;
mod dynamics;
mod parse;
mod statics;

fn main() -> Result<(), ()> {
    let source = "
    defn x(int). y(int,int). z(int,int).
    rule z(2,2).
    rule x(2). x(3). y(A,B) :- x(A), x(B).
    ";
    let parse_result = parse::program(source);
    dbg!(&parse_result);
    let (_input, program) = parse_result.map_err(drop)?;
    let check_result = program.check();
    dbg!(&check_result);
    let r2v2d = check_result.map_err(drop)?;
    dbg!(program.seal_break());
    let den = program.denotation(&r2v2d);
    println!("pos: {:?}\nunk: {:?}", &den.pos, &den.unk);
    Ok(())
}
