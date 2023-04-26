mod ast;
mod parse;
mod statics;

fn main() -> Result<(), ()> {
    let p = parse::program(
        "defn x(int). y(x,x,x). coolint(int).
        rule y(A,x(B),x(B)) :- x(B), A, x(C).",
    );
    println!("{:#?}", p);
    eprintln!("{:?}", p.map_err(drop)?.1.check());
    Ok(())
}
