mod ast;
mod parse;
mod statics;
mod well_formedness;

fn main() -> Result<(), ()> {
    let p = parse::program("defn x(int). y(x,x,x). rule y(A,x(B),x(B)) :- A,B,x(C).");
    println!("{:#?}", p);
    eprintln!("{:?}", p.map_err(drop)?.1.check());
    Ok(())
}
