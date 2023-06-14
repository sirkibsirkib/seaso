pub mod modules;
pub mod seaso;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, i64 as nomi64, multispace0, none_of, satisfy},
    combinator::{map as nommap, opt, recognize},
    error::ParseError,
    multi::{many0, many0_count, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
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
