use crate::*;
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::Hash,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Digraph<V: Ord> {
    verts: VecSet<V>,
    edges: VecSet<[V; 2]>,
}

/// Structure used in debug printing. Prints elements separated by commas.
pub(crate) struct CommaSep<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> {
    pub iter: I,
    pub spaced: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VecSet<T: Ord> {
    elements: Vec<T>,
}

// while this exists, the vecset has a violated invariant
pub struct VecSetMutGuard<'a, T: Ord> {
    set: &'a mut VecSet<T>,
}

/////////////////////////

impl<T: Ord> Default for Digraph<T> {
    fn default() -> Self {
        Digraph { edges: Default::default(), verts: Default::default() }
    }
}

impl<T: Ord> Digraph<T> {
    pub fn insert_edge(&mut self, edge: [T; 2])
    where
        T: Clone + Hash + Eq,
    {
        for vert in edge.iter() {
            self.insert_vert(vert.clone());
        }
        self.edges.insert(edge);
    }
    pub fn insert_vert(&mut self, vert: T)
    where
        T: Clone + Hash + Eq,
    {
        self.verts.insert(vert.clone());
    }

    pub fn contains_edge(&self, edge: &[T; 2]) -> bool
    where
        T: Hash + Eq,
    {
        self.edges.contains(edge)
    }
    pub fn transitively_close(&mut self)
    where
        T: Eq + Hash + Clone,
    {
        for x in self.verts.iter() {
            for y in self.verts.iter() {
                for z in self.verts.iter() {
                    if self.edges.contains(&[x.clone(), y.clone()])
                        && self.edges.contains(&[y.clone(), z.clone()])
                        && !self.edges.contains(&[x.clone(), z.clone()])
                    {
                        self.edges.insert([x.clone(), z.clone()]);
                    }
                }
            }
        }
    }
    pub fn verts(&self) -> &VecSet<T> {
        &self.verts
    }
}

pub fn map_snd<A, B, C>(
    iter: impl IntoIterator<Item = (A, B)>,
    b_to_c: impl Fn(B) -> C,
) -> impl Iterator<Item = (A, C)> {
    iter.into_iter().map(move |(a, b)| (a, b_to_c(b)))
}

pub fn sorted_vec<T: Ord>(iter: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut vec: Vec<T> = iter.into_iter().collect();
    vec.sort();
    vec
}

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

impl<T: Ord> IntoIterator for VecSet<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> std::vec::IntoIter<T> {
        self.elements.into_iter()
    }
}
impl<T: Ord> VecSet<T> {
    pub fn from_vec(mut elements: Vec<T>) -> Self {
        elements.sort();
        elements.dedup();
        Self { elements }
    }
    pub fn extend(&mut self, elements: impl IntoIterator<Item = T>) {
        for x in elements {
            self.insert(x);
        }
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
    pub fn contains(&self, t: &T) -> bool {
        self.elements.binary_search(&t).is_ok()
    }
    pub fn as_vec_mut(&mut self) -> VecSetMutGuard<T> {
        VecSetMutGuard { set: self }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_vec().iter()
    }
    pub fn as_vec(&self) -> &Vec<T> {
        &self.elements
    }
}
impl<'a, T: Ord> AsMut<Vec<T>> for VecSetMutGuard<'a, T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.set.elements
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

impl<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> Debug for CommaSep<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (i, x) in self.iter.clone().into_iter().enumerate() {
            if i > 0 {
                write!(f, "{}", if self.spaced { ", " } else { "," })?;
            }
            x.fmt(f)?;
        }
        Ok(())
    }
}

impl Debug for RuleAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Variable { vid, .. } => vid.fmt(f),
            Self::Constant(c) => c.fmt(f),
            Self::Construct { did, args } => {
                did.fmt(f)?;
                if !args.is_empty() {
                    write!(f, "({:?})", CommaSep { iter: args, spaced: false })?
                }
                Ok(())
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
            Statement::Decl(vec) => {
                write!(f, "decl ")?;
                for (i, did) in vec.iter().enumerate() {
                    if i > 0 {
                        write!(f, "=")?;
                    }
                    did.fmt(f)?;
                }
                Ok(())
            }
            Statement::Rule(Rule { consequents, antecedents }) => {
                write!(f, "rule {:?}", CommaSep { iter: consequents, spaced: true })?;
                if !antecedents.is_empty() {
                    write!(f, " :- {:?}", CommaSep { iter: antecedents, spaced: true })?;
                }
                Ok(())
            }
            Statement::Emit(did) => write!(f, "emit {:?}", did),
            Statement::Seal(did) => write!(f, "seal {:?}", did),
            Statement::Defn { did, params } => {
                write!(f, "defn {:?}({:?})", did, CommaSep { iter: params, spaced: false })
            }
        }
    }
}
impl Debug for DomainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}
impl Debug for VariableId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
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
