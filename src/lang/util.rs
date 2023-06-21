use crate::*;
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::Hash,
};
use std::collections::HashMap;

/// Newtype that suppresses pretty-printing of the wrapped type.
/// Useful in avoiding excessive indentation of internals when pretty printing its container.
pub(crate) struct NoPretty<T: Debug>(pub T);

/// Structure used in debug printing. Prints elements separated by commas.
pub(crate) struct CommaSep<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> {
    pub iter: I,
    pub spaced: bool,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VecSet<T: Ord> {
    elements: Vec<T>,
}

pub fn snd<A, B>((_, b): (A, B)) -> B {
    b
}

// while this exists, the vecset has a violated invariant
pub struct VecSetMutGuard<'a, T: Ord> {
    set: &'a mut VecSet<T>,
}

/////////////////////////
pub fn collect_map_lossless<K: Copy + Eq + Hash, V, I: IntoIterator<Item = (K, V)>>(
    iter: I,
) -> Result<HashMap<K, V>, K> {
    let mut map = HashMap::default();
    for (key, value) in iter.into_iter() {
        if map.insert(key, value).is_some() {
            return Err(key);
        }
    }
    Ok(map)
}

impl<'a, T: Ord> Drop for VecSetMutGuard<'a, T> {
    fn drop(&mut self) {
        self.set.elements.sort();
        self.set.elements.dedup();
    }
}
impl<T: Ord> Default for VecSet<T> {
    fn default() -> Self {
        Self { elements: vec![] }
    }
}
impl<'a, T: Ord> IntoIterator for VecSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}
impl<T: Ord> VecSet<T> {
    pub fn from_vec(mut elements: Vec<T>) -> Self {
        elements.sort();
        elements.dedup();
        Self { elements }
    }
    pub fn insert(&mut self, t: T) -> Option<T> {
        match self.elements.binary_search(&t) {
            Ok(idx) => {
                let mutref = unsafe {
                    // safe! just did range check
                    self.elements.get_unchecked_mut(idx)
                };
                Some(std::mem::replace(mutref, t))
            }
            Err(idx) => {
                self.elements.insert(idx, t);
                None
            }
        }
    }
    pub fn as_slice_mut(&mut self) -> VecSetMutGuard<T> {
        VecSetMutGuard { set: self }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
    }
}
impl<'a, T: Ord> AsMut<[T]> for VecSetMutGuard<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.set.elements.as_mut()
    }
}
impl<T: Ord> FromIterator<T> for VecSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut x = Self::default();
        for q in iter {
            x.insert(q);
        }
        x
    }
}

impl<T: Debug> Debug for NoPretty<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}

impl<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> Debug for CommaSep<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (i, x) in self.iter.clone().into_iter().enumerate() {
            if i > 0 {
                write!(f, "{}", if self.spaced { ", " } else { "," })?;
            }
            write!(f, "{:?}", x)?;
        }
        Ok(())
    }
}

impl Debug for RuleAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Variable(vid) => vid.fmt(f),
            Self::Constant(c) => c.fmt(f),
            Self::Construct { did, args } => {
                write!(f, "{:?}({:?})", did, CommaSep { iter: args, spaced: false })
            }
        }
    }
}
impl Debug for RuleLiteral {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.sign == Sign::Neg {
            write!(f, "!")?
        }
        self.ra.fmt(f)
    }
}

impl Debug for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Statement::Rule(Rule { consequents, antecedents }) => {
                write!(f, "rule {:?}", CommaSep { iter: consequents, spaced: true })?;
                if !antecedents.is_empty() {
                    write!(f, " :- {:?}", CommaSep { iter: antecedents, spaced: true })?;
                }
                Ok(())
            }
            Statement::Decl(did) => write!(f, "decl {:?}", did),
            Statement::Emit(did) => write!(f, "emit {:?}", did),
            Statement::Seal(did) => write!(f, "seal {:?}", did),
            Statement::Defn { did, params } => {
                write!(f, "defn {:?}({:?})", did, CommaSep { iter: params, spaced: false })
            }
        }?;
        write!(f, ". ")
    }
}
impl Debug for DomainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}

impl Debug for Constant {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Int(c) => c.fmt(f),
            Self::Str(c) => c.fmt(f),
        }
    }
}
impl Debug for VariableId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}
