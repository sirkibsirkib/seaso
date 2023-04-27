mod ast;
mod dynamics;
mod parse;
mod statics;

fn main() -> Result<(), ()> {
    let p = parse::program(
        "
        seal x. emit x.
        defn x(int). y(x,x,x). coolint(int).
        rule y(A,x(B),x(B)) :- x(B), A, x(C).
        ",
    );
    println!("{:#?}", p);
    let p = p.map_err(drop)?.1;
    let t = p.check();
    eprintln!("{:?}", t);
    eprintln!("{:?}", p.seal_break());
    Ok(())
}
