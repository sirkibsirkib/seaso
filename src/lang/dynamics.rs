use crate::*;
use core::fmt::Debug;
use std::collections::{HashMap, HashSet};

/// Concrete counterpart to `RuleAtom` with no domain info
#[derive(Ord, PartialOrd, Hash, Eq, PartialEq, Clone)]
pub enum Atom {
    Constant { c: Constant },
    Construct { did: DomainId, args: Vec<Atom> },
}

/// A store of atoms, grouped by domain for ease of lookup.
#[derive(Default, PartialEq, Eq)]
pub struct Knowledge {
    pub map: HashMap<DomainId, HashSet<Atom>>,
}

/// Three instances of `Knowledge`, denoting truths, unknowns, and emissions.
/// Invariants:
/// 1. truths and unknowns are disjoint.
/// 2. emissions are a subset of truths.
#[derive(Debug)]
pub struct Denotation {
    pub truths: Knowledge,
    pub unknowns: Knowledge,
    pub emissions: Knowledge,
}

/// Used internally when concretizing a rule. Conceptually, is a map from VariableId to Atom.
/// Here, implemented with a vector so that the state of its mappings can be easily saved and reverted.
#[derive(Debug, Default)]
pub struct VariableAssignments {
    assignments: Vec<(VariableId, Atom)>,
}

/// Encodes a snapshot of a growing `VariableAssignments` structure. Used to revert prior states.
struct StateToken {
    assignments_count: usize,
}

/// Represent an immutable `Knowledge` value whose contents can be logically negated.
#[derive(Debug, Copy, Clone)]
pub enum ComplementKnowledge<'a> {
    Empty,
    ComplementOf(&'a Knowledge),
}

pub trait Executable {
    fn annotated_rules(&self) -> &[AnnotatedRule];
    fn emissive(&self, did: &DomainId) -> bool;
    fn starting_facts(&self) -> Knowledge;
    fn big_step_inference(
        &self,
        neg: ComplementKnowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
    ) -> Knowledge {
        let mut pos_r = self.starting_facts();
        loop {
            for AnnotatedRule { v2d, rule } in self.annotated_rules() {
                rule.inference_stage(v2d, neg, &pos_r, pos_w, va)
            }
            let changed = pos_r.absorb_all(pos_w);
            if !changed {
                assert!(pos_w.map.is_empty());
                return pos_r;
            }
        }
    }

    fn denotation(&self) -> Denotation {
        let mut pos_w = self.starting_facts();
        let mut va = VariableAssignments::default();
        let mut interpretations =
            vec![self.big_step_inference(ComplementKnowledge::Empty, &mut pos_w, &mut va)];
        loop {
            if interpretations.len() % 2 == 1 {
                if let [.., a, b, c, d] = interpretations.as_mut_slice() {
                    if a == c && b == d {
                        use std::mem::take;
                        let truths = take(d);
                        let mut unknowns = take(c);
                        for (did, set) in unknowns.map.iter_mut() {
                            set.retain(|atom| !truths.contains(did, atom))
                        }
                        let emissions = Knowledge {
                            map: truths
                                .map
                                .iter()
                                .filter_map(|(did, set)| {
                                    if self.emissive(did) {
                                        Some((did.clone(), set.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        };
                        return Denotation { truths, unknowns, emissions };
                    }
                }
            }
            let neg = ComplementKnowledge::ComplementOf(interpretations.iter().last().unwrap());
            assert!(pos_w.map.is_empty());
            assert!(va.assignments.is_empty());
            let pos = self.big_step_inference(neg, &mut pos_w, &mut va);
            interpretations.push(pos);
        }
    }
}

//////////////////////////////////////////////////////

impl Executable for ExecutableProgram {
    fn annotated_rules(&self) -> &[AnnotatedRule] {
        &self.annotated_rules
    }
    fn emissive(&self, did: &DomainId) -> bool {
        self.emissive.contains(did)
    }
    fn starting_facts(&self) -> Knowledge {
        Default::default()
    }
}

impl Executable for (&[Atom], &ExecutableProgram) {
    fn annotated_rules(&self) -> &[AnnotatedRule] {
        self.1.annotated_rules()
    }
    fn emissive(&self, did: &DomainId) -> bool {
        self.1.emissive(did)
    }
    fn starting_facts(&self) -> Knowledge {
        let mut k = Knowledge::default();
        for atom in self.0 {
            k.insert(atom.domain_id(), atom.clone());
        }
        k
    }
}

impl ComplementKnowledge<'_> {
    fn contains(self, did: &DomainId, atom: &Atom) -> bool {
        match self {
            Self::Empty => false,
            Self::ComplementOf(k) => !k.contains(did, atom),
        }
    }
}

impl std::fmt::Debug for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constant { c } => match c {
                Constant::Int(c) => c.fmt(f),
                Constant::Str(c) => c.fmt(f),
            },
            Self::Construct { did, args } => {
                let mut f = f.debug_tuple(&did.0);
                for arg in args {
                    f.field(arg);
                }
                f.finish()
            }
        }
    }
}

impl std::fmt::Debug for Knowledge {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let vec_map: HashMap<&DomainId, Vec<_>> = self
            .map
            .iter()
            .filter_map(|(did, set)| {
                if set.is_empty() {
                    None
                } else {
                    Some({
                        let mut vec = set.iter().map(super::util::NoPretty).collect::<Vec<_>>();
                        vec.sort_by_key(|t| t.0);
                        (did, vec)
                    })
                }
            })
            .collect();
        f.debug_map().entries(vec_map).finish()
    }
}

impl VariableAssignments {
    fn get_state_token(&self) -> StateToken {
        StateToken { assignments_count: self.assignments.len() }
    }
    fn restore_state(&mut self, state_token: StateToken) -> Result<(), ()> {
        if state_token.assignments_count <= self.assignments.len() {
            self.assignments.truncate(state_token.assignments_count);
            Ok(())
        } else {
            Err(())
        }
    }
    fn insert(&mut self, vid: &VariableId, atom2: Atom) -> Result<(), ()> {
        if let Some(atom1) = self.get_mut(vid) {
            if atom1 == &atom2 {
                Ok(())
            } else {
                Err(())
            }
        } else {
            self.assignments.push((vid.clone(), atom2));
            Ok(())
        }
    }
    fn get_mut(&mut self, vid: &VariableId) -> Option<&mut Atom> {
        self.assignments
            .iter_mut()
            .filter_map(|pair| if &pair.0 == vid { Some(&mut pair.1) } else { None })
            .next()
    }
    fn get(&self, vid: &VariableId) -> Option<&Atom> {
        self.assignments
            .iter()
            .filter_map(|pair| if &pair.0 == vid { Some(&pair.1) } else { None })
            .next()
    }
}

impl Knowledge {
    pub fn atoms_in_domain(&self, did: &DomainId) -> impl Iterator<Item = &Atom> + '_ {
        self.map.get(did).into_iter().flat_map(|set| set.iter())
    }
    pub fn contains(&self, did: &DomainId, atom: &Atom) -> bool {
        self.map.get(did).map(|set| set.contains(atom)).unwrap_or(false)
    }
    pub fn insert(&mut self, did: &DomainId, atom: Atom) -> bool {
        self.map.entry(did.clone()).or_default().insert(atom)
    }
    pub fn absorb_all(&mut self, other: &mut Self) -> bool {
        let mut changed = false;
        for (did, set) in other.map.drain() {
            for atom in set {
                if self.insert(&did, atom) {
                    changed = true;
                }
            }
        }
        changed
    }
}

impl Atom {
    fn domain_id(&self) -> &DomainId {
        match self {
            Atom::Construct { did, .. } => did,
            Atom::Constant { c } => c.domain_id(),
        }
    }
    fn uniquely_assign_variables(
        &self,
        ra: &RuleAtom,
        va: &mut VariableAssignments,
    ) -> Result<(), ()> {
        match (self, ra) {
            (atom1, RuleAtom::Variable(vid)) => va.insert(vid, atom1.clone()),
            (Atom::Construct { args: atoms, .. }, RuleAtom::Construct { args: rule_atoms, .. }) => {
                if atoms.len() == rule_atoms.len() {
                    for (atom, rule_atom) in atoms.iter().zip(rule_atoms) {
                        atom.uniquely_assign_variables(rule_atom, va)?
                    }
                    Ok(())
                } else {
                    Err(())
                }
            }
            (Atom::Constant { c: c1 }, RuleAtom::Constant(c2)) => {
                if c1 == c2 {
                    Ok(())
                } else {
                    Err(())
                }
            }
            (x, y) => panic!("{:?}", (x, y)),
        }
    }
}

impl RuleAtom {
    fn concretize(&self, va: &VariableAssignments) -> Result<Atom, ()> {
        match self {
            RuleAtom::Variable(vid) => va.get(vid).ok_or(()).cloned(),
            RuleAtom::Constant(c) => Ok(Atom::Constant { c: c.clone() }),
            RuleAtom::Construct { args, did } => Ok(Atom::Construct {
                args: args.iter().map(|ra| ra.concretize(va)).collect::<Result<Vec<_>, _>>()?,
                did: did.clone(),
            }),
        }
    }
}

impl Rule {
    fn inference_stage(
        &self,
        v2d: &VariableTypes,
        neg: ComplementKnowledge,
        pos_r: &Knowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
    ) {
        // println!("inference with va {:?}", va);
        self.inference_stage_rec(v2d, neg, pos_r, pos_w, va, &self.antecedents);
        va.assignments.clear();
    }

    fn inference_stage_rec(
        &self,
        v2d: &VariableTypes,
        neg: ComplementKnowledge,
        pos_r: &Knowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
        tail: &[RuleLiteral],
    ) {
        match tail {
            [] => {
                // perform all checks
                let checks_pass = self.antecedents.iter().all(|antecedent| {
                    if antecedent.sign == Sign::Neg {
                        let did = antecedent.ra.domain_id(v2d).expect("static checked");
                        let atom = antecedent.ra.concretize(va).expect("should work");
                        neg.contains(&did, &atom)
                    } else {
                        true
                    }
                });
                if checks_pass {
                    // all checks passed
                    for consequent in self.consequents.iter() {
                        let did = consequent.domain_id(v2d).expect("static checked");
                        let atom = consequent.concretize(va).expect("should work");
                        pos_w.insert(&did, atom);
                    }
                }
            }
            [head, new_tail @ ..] => match head.sign {
                Sign::Pos => {
                    let did = head.ra.domain_id(v2d).expect("BAD");
                    for atom in pos_r.atoms_in_domain(&did) {
                        let state_token = va.get_state_token();
                        if atom.uniquely_assign_variables(&head.ra, va).is_ok() {
                            self.inference_stage_rec(v2d, neg, pos_r, pos_w, va, new_tail)
                        }
                        va.restore_state(state_token).expect("oh no");
                    }
                }
                Sign::Neg => self.inference_stage_rec(v2d, neg, pos_r, pos_w, va, new_tail),
            },
        }
    }
}
