use crate::lang::VecSet;
use crate::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, i64 as nomi64, multispace0, none_of, satisfy},
    combinator::{map as nommap, opt, recognize, verify},
    error::ParseError,
    multi::{many0, many0_count, many1, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
};
pub type IResult<I, O, E = nom::error::VerboseError<I>> = Result<(I, O), nom::Err<E>>;

////////// PARSER COMBINATORS //////////

pub fn wsl<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    preceded(multispace0, inner)
}

pub fn wstag<'a, E>(inner: &'a str) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + 'a,
{
    wsl(tag(inner))
}

pub fn wsr<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    terminated(inner, multispace0)
}

pub fn commasep<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    E: ParseError<&'a str> + 'a,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
    O: 'a,
{
    separated_list0(wstag(","), inner)
}

pub fn list<'a, F: 'a, O: 'a, E: ParseError<&'a str> + 'a>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    delimited(wstag("("), commasep(inner), wstag(")"))
}

pub fn all_consuming<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    nom::combinator::all_consuming(wsr(inner))
}

////////////////////////////

pub fn program(i: &str) -> IResult<&str, Program> {
    enum X {
        Statements(Vec<Statement>),
        Part(Part),
    }
    let sta = nommap(statements1, X::Statements);
    let par = nommap(part, X::Part);
    let f = |xs: Vec<X>| {
        let mut anon_mod_statements = Vec::<Statement>::default();
        let mut parts = VecSet::default();
        for x in xs {
            match x {
                X::Statements(s) => anon_mod_statements.extend(s),
                X::Part(p) => drop(parts.insert(p)),
            }
        }
        Program { anon_mod_statements, parts }
    };
    nommap(many0(alt((sta, par))), f)(i)
}

pub fn part(i: &str) -> IResult<&str, Part> {
    let name = preceded(wstag("part"), part_name);
    let uses = preceded(wstag(":"), commasep(part_name));
    let body = delimited(wstag("{"), statements0, wstag("}"));
    let p = tuple((name, opt(uses), body));
    nommap(p, |(name, maybe_uses, statements)| Part {
        name,
        uses: VecSet::from_vec(maybe_uses.unwrap_or_default()),
        statements: VecSet::from_vec(statements),
    })(i)
}
pub fn like_statements(i: &str) -> IResult<&str, Vec<Statement>> {
    pub fn stmts1<'a, F: FnMut(&'a str) -> IResult<&'a str, Statement> + 'a>(
        string: &'a str,
        inner: F,
    ) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<Statement>> + 'a {
        preceded(wstag(string), many0(terminated(inner, wstag("."))))
    }
    alt((
        stmts1("decl", decl),
        stmts1("defn", defn),
        stmts1("seal", seal),
        stmts1("emit", emit),
        stmts1("rule", rule),
    ))(i)
}
pub fn statements0(i: &str) -> IResult<&str, Vec<Statement>> {
    nommap(many0(like_statements), |x| x.into_iter().flatten().collect())(i)
}
pub fn statements1(i: &str) -> IResult<&str, Vec<Statement>> {
    nommap(many1(like_statements), |x| x.into_iter().flatten().collect())(i)
}

pub fn decl(i: &str) -> IResult<&str, Statement> {
    let p = separated_list0(wstag("="), domain_id);
    nommap(p, Statement::Decl)(i)
}
pub fn emit(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, Statement::Emit)(i)
}

pub fn seal(i: &str) -> IResult<&str, Statement> {
    nommap(domain_id, Statement::Seal)(i)
}

pub fn defn(i: &str) -> IResult<&str, Statement> {
    let p = pair(domain_id, opt(list(domain_id)));
    nommap(p, |(did, maybe_params)| Statement::Defn {
        did,
        params: maybe_params.unwrap_or_default(),
    })(i)
}

pub fn rule(i: &str) -> IResult<&str, Statement> {
    let c = commasep(rule_atom);
    let a = alt((
        preceded(wstag(":-"), commasep(rule_literal)),
        nommap(multispace0, |_| Vec::default()),
    ));
    nommap(pair(c, a), |(consequents, antecedents)| {
        Statement::Rule(Rule { consequents, antecedents })
    })(i)
}

////////// (SUB)EXPRESSION-LEVEL PARSERS //////////

pub fn id_suffix(i: &str) -> IResult<&str, &str> {
    recognize(many0_count(alt((tag("_"), tag("-"), alphanumeric1))))(i)
}

pub fn domain_id(i: &str) -> IResult<&str, DomainId> {
    let pre = many0_count(tag("_"));
    let fst = pair(satisfy(|c| c.is_ascii_lowercase()), id_suffix);
    let snd = pair(wsl(tag("@")), part_name);
    let did = recognize(tuple((pre, fst, opt(snd))));
    nommap(wsl(did), |ident| DomainId(ident.to_owned()))(i)
}

pub fn ascription(i: &str) -> IResult<&str, DomainId> {
    preceded(wstag(":"), domain_id)(i)
}

pub fn variable(i: &str) -> IResult<&str, RuleAtom> {
    let some_vid =
        recognize(tuple((many0_count(tag("_")), satisfy(|c| c.is_ascii_uppercase()), id_suffix)));
    let vid = wsl(alt((some_vid, tag("_"))));
    let variable_id = nommap(vid, |ident| VariableId(ident.to_owned()));
    nommap(pair(variable_id, opt(ascription)), |(vid, ascription)| RuleAtom::Variable {
        vid,
        ascription,
    })(i)
}

pub fn string(i: &str) -> IResult<&str, String> {
    nommap(delimited(wstag("\""), recognize(many0_count(none_of("\""))), tag("\"")), str::to_owned)(
        i,
    )
}

pub fn part_name(i: &str) -> IResult<&str, PartName> {
    let some_mid = wsl(recognize(pair(satisfy(|c| c.is_ascii_lowercase()), id_suffix)));
    nommap(nommap(some_mid, str::to_owned), PartName)(i)
}

pub fn constant(i: &str) -> IResult<&str, RuleAtom> {
    let int_constant = nommap(nomi64, Constant::Int);
    let str_constant = nommap(string, Constant::Str);
    nommap(wsl(alt((int_constant, str_constant))), RuleAtom::Constant)(i)
}

pub fn construct(i: &str) -> IResult<&str, RuleAtom> {
    let f = |did: &DomainId| did.0 != "str" && did.0 != "int";
    let pair = pair(verify(domain_id, f), opt(list(rule_atom)));
    nommap(pair, |(did, maybe_args)| RuleAtom::Construct {
        did,
        args: maybe_args.unwrap_or_default(),
    })(i)
}

pub fn rule_atom(i: &str) -> IResult<&str, RuleAtom> {
    alt((construct, variable, constant))(i)
}

pub fn rule_literal(i: &str) -> IResult<&str, RuleLiteral> {
    let (i, (excl, ra)) = pair(opt(wstag("!")), rule_atom)(i)?;
    let sign = if excl.is_some() { Sign::Neg } else { Sign::Pos };
    Ok((i, RuleLiteral { sign, ra }))
}
