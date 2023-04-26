use crate::ast::*;
use nom::character::complete::multispace0;
use nom::combinator::map as nommap;
use nom::error::ParseError;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::terminated;

use nom::{branch::alt, bytes::complete::tag, character::complete::alpha1, multi::many0, IResult};

fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn program(mut i: &str) -> IResult<&str, Program> {
    let mut program = Program::default();
    while !i.is_empty() {
        let (i2, statements) = alt((
            preceded(ws(tag("decl")), many0(terminated(decl, ws(tag("."))))),
            preceded(ws(tag("defn")), many0(terminated(defn, ws(tag("."))))),
        ))(i)?;
        i = i2;
        program.statements.extend(statements)
    }
    Ok((i, program))
}

fn domain_id(i: &str) -> IResult<&str, DomainId> {
    let (i, ident) = ws(alpha1)(i)?;
    Ok((i, DomainId(ident.to_owned())))
}

fn decl(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Decl { did })(i)
}

fn defn(i: &str) -> IResult<&str, Statement> {
    let (i, did) = domain_id(i)?;
    let (i, params) =
        delimited(ws(tag("(")), separated_list0(ws(tag(",")), ws(domain_id)), ws(tag(")")))(i)?;
    Ok((i, Statement::Defn { did, params }))
}

fn rule(i: &str) -> IResult<&str, Statement> {
    // tag("X")(i)
    todo!()
}

fn emit(i: &str) -> IResult<&str, Statement> {
    todo!()
}

fn seal(i: &str) -> IResult<&str, Statement> {
    todo!()
}

// fn parse_program(mut i: &str) -> Result<(&str, Program), &str> {
//     let mut program = Program::default();
//     while !i.is_empty() {
//         let (i2, statement) = parse_statement(i)?;
//         i = i2;
//         program.statements.push(statement);
//     }
//     Ok((i, program))
// }

// fn parse_statement(mut i: &str) -> Result<(&str, Statement), &str> {

// }

// fn parse_decl(mut i: &str) -> Result<(&str, Statement), &str> {

// }
// fn parse_defn(mut i: &str) -> Result<(&str, Statement), &str> {

// }
// fn parse_emit(mut i: &str) -> Result<(&str, Statement), &str> {

// }
// fn parse_rule(mut i: &str) -> Result<(&str, Statement), &str> {

// }

pub fn test() {
    let p = program("decl     bim. bop. bow. defn whee(wah,wang). woo().");
    println!("{:#?}", p);
}
