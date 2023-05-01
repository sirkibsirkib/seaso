use crate::ast::*;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::i64 as nomi64,
        complete::{alphanumeric1, multispace0, none_of, satisfy},
    },
    combinator::{map as nommap, opt, recognize},
    error::ParseError,
    multi::{many0, many0_count, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};

////////// PARSER COMBINATORS //////////

fn wsl<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    preceded(multispace0, inner)
}

fn wsr<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    terminated(inner, multispace0)
}

fn ws<'a, F: 'a, O: 'a, E: ParseError<&'a str> + 'a>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    wsl(wsr(inner))
}

fn commasep<'a, F: 'a, O: 'a, E: ParseError<&'a str> + 'a>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    separated_list0(ws(tag(",")), inner)
}

fn list<'a, F: 'a, O: 'a, E: ParseError<&'a str> + 'a>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(ws(tag("(")), commasep(inner), ws(tag(")")))
}

fn stmt<'a, F: FnMut(&'a str) -> IResult<&'a str, Statement> + 'a>(
    string: &'a str,
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<Statement>> + 'a {
    preceded(ws(tag(string)), many0(terminated(inner, ws(tag(".")))))
}

////////// STATEMENT PARSERS //////////

pub fn program(mut i: &str) -> IResult<&str, Program> {
    let mut program = Program::default();
    while !i.is_empty() {
        let (i2, statements) = alt((
            stmt("decl", decl),
            stmt("defn", defn),
            stmt("seal", seal),
            stmt("emit", emit),
            stmt("rule", rule),
        ))(i)?;
        i = i2;
        program.statements.extend(statements)
    }
    Ok((i, program))
}

fn decl(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Decl { did })(i)
}
fn emit(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Emit { did })(i)
}

fn seal(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, |did| Statement::Seal { did })(i)
}

fn defn(i: &str) -> IResult<&str, Statement> {
    let (i, did) = domain_id(i)?;
    let (i, params) = list(domain_id)(i)?;
    Ok((i, Statement::Defn { did, params }))
}

fn rule(i: &str) -> IResult<&str, Statement> {
    let (i, consequents) = commasep(rule_atom)(i)?;
    let (i, antecedents) = alt((
        preceded(ws(tag(":-")), commasep(rule_literal)),
        nommap(multispace0, |_| Vec::default()),
    ))(i)?;
    Ok((i, Statement::Rule(Rule { consequents, antecedents })))
}

////////// (SUB)EXPRESSION PARSERS //////////

fn id_suffix(i: &str) -> IResult<&str, &str> {
    recognize(many0_count(alt((tag("_"), alphanumeric1))))(i)
}

fn domain_id(i: &str) -> IResult<&str, DomainId> {
    let did = recognize(pair(satisfy(|c| c.is_ascii_lowercase()), id_suffix));
    nommap(ws(did), |ident| DomainId(ident.to_owned()))(i)
}

fn variable(i: &str) -> IResult<&str, RuleAtom> {
    let vid = recognize(pair(satisfy(|c| c.is_ascii_uppercase()), id_suffix));
    let variable_id = nommap(ws(vid), |ident| VariableId(ident.to_owned()));
    nommap(variable_id, |vid| RuleAtom::Variable { vid })(i)
}

fn constant(i: &str) -> IResult<&str, RuleAtom> {
    let int_constant = nommap(nomi64, |c| Constant::Int(c));
    let str_constant = nommap(
        delimited(tag("\""), recognize(many0_count(none_of("\""))), tag("\"")),
        |c: &str| Constant::Str(c.to_owned()),
    );
    nommap(ws(alt((int_constant, str_constant))), |c| RuleAtom::Constant { c })(i)
}

fn construct(i: &str) -> IResult<&str, RuleAtom> {
    let (i, (did, args)) = pair(domain_id, list(rule_atom))(i)?;
    Ok((i, RuleAtom::Construct { did, args }))
}

fn rule_atom(i: &str) -> IResult<&str, RuleAtom> {
    alt((variable, constant, construct))(i)
}

fn rule_literal(i: &str) -> IResult<&str, RuleLiteral> {
    let (i, (excl, ra)) = pair(opt(ws(tag("!"))), rule_atom)(i)?;
    let sign = if excl.is_some() { Sign::Neg } else { Sign::Pos };
    Ok((i, RuleLiteral { sign, ra }))
}
