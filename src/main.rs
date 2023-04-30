mod ast;
// mod data_structures;
mod dynamics;
mod parse;
mod statics;

fn main() -> Result<(), ()> {
    let source = "
    defn x(int). y(int).
    rule x(2).
    rule y(2) :- x(2).
    ";
    let parse_result = parse::program(source);
    println!("{:#?}", parse_result);
    let (_input, program) = parse_result.map_err(drop)?;
    let check_result = program.check();
    println!("{:?}", check_result);
    let r2v2d = check_result.map_err(drop)?;
    println!("{:?}", program.seal_break());
    let neg = dynamics::Knowledge::default();
    let pos = program.big_step_inference(&r2v2d, &neg);
    println!("{:?}", pos);
    Ok(())
}
