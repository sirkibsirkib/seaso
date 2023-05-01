mod ast;
// mod data_structures;
mod dynamics;
mod parse;
mod statics;

fn main() -> Result<(), ()> {
    // let source = "
    // defn x(int). y(int,int). z(int,int).
    // rule z(1,0). y(A,B) :- x(A), x(B), !z(A,B).
    //      x(L), x(R) :- z(L,R).
    // ";
    let source = "
    defn x(int).
    rule x(1) :- x(3), !x(2).
         x(2) :- !x(1).
         x(3).
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
