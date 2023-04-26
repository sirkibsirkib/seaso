use crate::ast::*;
use nom::character::complete::i64 as nomi64;
use nom::character::complete::multispace0;
use nom::combinator::map as nommap;
use nom::combinator::verify;
use nom::error::ParseError;
use nom::multi::many1;
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
            preceded(ws(tag("seal")), many0(terminated(seal, ws(tag("."))))),
            preceded(ws(tag("emit")), many0(terminated(emit, ws(tag("."))))),
        ))(i)?;
        i = i2;
        program.statements.extend(statements)
    }
    Ok((i, program))
}

fn domain_id(i: &str) -> IResult<&str, DomainId> {
    let (i, ident) = verify(ws(alpha1), |s: &str| {
        s.chars().filter(|c: &char| c.is_lowercase()).next().is_some()
    })(i)?;
    Ok((i, DomainId(ident.to_owned())))
}

fn variable_id(i: &str) -> IResult<&str, VariableId> {
    let (i, ident) = verify(ws(alpha1), |s: &str| {
        s.chars().filter(|c: &char| c.is_uppercase()).next().is_some()
    })(i)?;
    Ok((i, VariableId(ident.to_owned())))
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

fn variable(i: &str) -> IResult<&str, RuleAtom> {
    nommap(variable_id, |vid| RuleAtom::Variable { vid })(i)
}

fn int_const(i: &str) -> IResult<&str, RuleAtom> {
    nommap(ws(nomi64), |c| RuleAtom::IntConst { c })(i)
}

fn str_cosnt(i: &str) -> IResult<&str, RuleAtom> {}

fn construct(i: &str) -> IResult<&str, RuleAtom> {}

fn rule_atom(i: &str) -> IResult<&str, RuleAtom> {
    alt((variable, int_const, str_cosnt, construct))(i)
}
fn rule_literal(i: &str) -> IResult<&str, RuleLiteral> {
    todo!()
}

fn rule(i: &str) -> IResult<&str, Statement> {
    let (i, consequents) = many1(rule_atom)(i)?;
    let (i, maybe_antecedents) =
        alt((nommap(many0(rule_literal), |x| Some(x)), nommap(multispace0, |x| None)))(i)?;
    todo!()
}

fn emit(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Emit { did })(i)
}

fn seal(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Seal { did })(i)
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
    let p = program("decl     bim. bop. bow. defn whee(wah,wang). woo(). seal woo.");
    println!("{:#?}", p);
}
