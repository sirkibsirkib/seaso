use crate::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, i64 as nomi64, multispace0, none_of, satisfy},
    combinator::{map as nommap, opt, recognize},
    error::ParseError,
    multi::{many0, many0_count, many1, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

////////// PARSER COMBINATORS //////////

pub fn wsl<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    preceded(multispace0, inner)
}

pub fn wsr<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    terminated(inner, multispace0)
}

pub fn ws<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str> + 'a,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
    O: 'a,
{
    wsl(wsr(inner))
}

pub fn commasep<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    E: ParseError<&'a str> + 'a,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
    O: 'a,
{
    separated_list0(ws(tag(",")), inner)
}

pub fn list<'a, F: 'a, O: 'a, E: ParseError<&'a str> + 'a>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    delimited(ws(tag("(")), commasep(inner), ws(tag(")")))
}

////////////////////////////

pub fn modules(i: &str) -> IResult<&str, Vec<Module>> {
    many1(module)(i)
}

pub fn module(i: &str) -> IResult<&str, Module> {
    let (i, (_, name, _, uses, _, statements, _)) = tuple((
        ws(tag("module")),
        module_name,
        ws(tag(":")),
        commasep(module_name),
        ws(tag("{")),
        statements,
        ws(tag("}")),
    ))(i)?;
    Ok((i, Module { name, uses: uses.into_iter().collect(), statements }))
}

pub fn statements(i: &str) -> IResult<&str, Statements> {
    pub fn stmt<'a, F: FnMut(&'a str) -> IResult<&'a str, Statement> + 'a>(
        string: &'a str,
        inner: F,
    ) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<Statement>> + 'a {
        preceded(ws(tag(string)), many0(terminated(inner, ws(tag(".")))))
    }
    let like_statements = alt((
        stmt("decl", decl),
        stmt("defn", defn),
        stmt("seal", seal),
        stmt("emit", emit),
        stmt("rule", rule),
    ));
    nommap(many0(like_statements), |x: Vec<Vec<Statement>>| {
        Statements(x.into_iter().flatten().collect())
    })(i)
}

pub fn decl(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, Statement::Decl)(i)
}
pub fn emit(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, Statement::Emit)(i)
}

pub fn seal(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, Statement::Seal)(i)
}

pub fn defn(i: &str) -> IResult<&str, Statement> {
    let (i, did) = domain_id(i)?;
    let (i, params) = list(domain_id)(i)?;
    Ok((i, Statement::Defn { did, params }))
}

pub fn rule(i: &str) -> IResult<&str, Statement> {
    let c = commasep(rule_atom);
    let a = alt((
        preceded(ws(tag(":-")), commasep(rule_literal)),
        nommap(multispace0, |_| Vec::default()),
    ));
    let (i, (consequents, antecedents)) = pair(c, a)(i)?;
    Ok((i, Statement::Rule(Rule { consequents, antecedents })))
}

////////// (SUB)EXPRESSION-LEVEL PARSERS //////////

pub fn id_suffix(i: &str) -> IResult<&str, &str> {
    recognize(many0_count(alt((tag("_"), alphanumeric1))))(i)
}

pub fn domain_id(i: &str) -> IResult<&str, DomainId> {
    let did = recognize(pair(satisfy(|c| c.is_ascii_lowercase()), id_suffix));
    nommap(ws(did), |ident| DomainId(ident.to_owned()))(i)
}

pub fn variable(i: &str) -> IResult<&str, RuleAtom> {
    let some_vid = recognize(pair(satisfy(|c| c.is_ascii_uppercase()), id_suffix));
    let vid = alt((some_vid, tag("_")));
    let variable_id = nommap(ws(vid), |ident| VariableId(ident.to_owned()));
    nommap(variable_id, RuleAtom::Variable)(i)
}

pub fn string(i: &str) -> IResult<&str, String> {
    nommap(delimited(tag("\""), recognize(many0_count(none_of("\""))), tag("\"")), str::to_owned)(i)
}

pub fn module_name(i: &str) -> IResult<&str, ModuleName> {
    let p = nommap(recognize(many0_count(alphanumeric1)), str::to_owned);
    nommap(p, ModuleName)(i)
}

pub fn constant(i: &str) -> IResult<&str, RuleAtom> {
    let int_constant = nommap(nomi64, Constant::Int);
    let str_constant = nommap(string, Constant::Str);
    nommap(ws(alt((int_constant, str_constant))), RuleAtom::Constant)(i)
}

pub fn construct(i: &str) -> IResult<&str, RuleAtom> {
    let pair = pair(domain_id, list(rule_atom));
    nommap(pair, |(did, args)| RuleAtom::Construct { did, args })(i)
}

pub fn rule_atom(i: &str) -> IResult<&str, RuleAtom> {
    alt((variable, constant, construct))(i)
}

pub fn rule_literal(i: &str) -> IResult<&str, RuleLiteral> {
    let (i, (excl, ra)) = pair(opt(ws(tag("!"))), rule_atom)(i)?;
    let sign = if excl.is_some() { Sign::Neg } else { Sign::Pos };
    Ok((i, RuleLiteral { sign, ra }))
}
