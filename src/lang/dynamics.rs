use crate::*;
use core::fmt::Debug;
use std::collections::{HashMap, HashSet};

/// Concrete counterpart to `RuleAtom` with no domain info
#[derive(Ord, PartialOrd, Hash, Eq, PartialEq, Clone)]
pub enum Atom {
    Constant { c: Constant },
    Construct { did: DomainId, args: Vec<Atom> },
}

pub struct Bare<T>(T);

/// A store of atoms, grouped by domain for ease of lookup.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Knowledge {
    pub map: HashMap<DomainId, HashSet<Atom>>,
}

/// Three instances of `Knowledge`, denoting truths, unknowns, and emissions.
/// Invariants:
/// 1. truths and unknowns are disjoint.
/// 2. emissions are a subset of truths.
pub struct DenotationResult {
    pub denotation: Denotation<Knowledge>,
    pub prev_truths: Knowledge,
}
#[derive(Debug)]
pub struct Denotation<T: Debug> {
    pub truths: T,
    pub unknowns: T,
    pub emissions: T,
}

/// Used internally when concretizing a rule. Conceptually, is a map from VariableId to Atom.
/// Here, implemented with a vector so that the state of its mappings can be easily saved and reverted.
#[derive(Debug, Default)]
pub struct VariableAssignments {
    assignments: Vec<(VariableId, Atom)>,
}

pub struct Literal {
    pub sign: Sign,
    pub atom: Atom,
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
#[derive(Debug)]
pub struct ConcreteInference {
    pub consequent: Atom,
    pub antecedents: Vec<Literal>,
}

impl RuleLiteral {
    fn concretize(&self, va: &VariableAssignments) -> Result<Literal, ()> {
        self.ra.concretize(va).map(|atom| Literal { sign: self.sign.clone(), atom })
    }
}
impl ExecutableProgram {
    pub fn how(&self, dr: &DenotationResult) -> Vec<ConcreteInference> {
        let mut pos_w = Knowledge::default();
        let mut va = VariableAssignments::default();
        let mut concrete_inferences = Vec::<ConcreteInference>::default();
        let neg = ComplementKnowledge::ComplementOf(&dr.prev_truths);
        let _ = self.big_step_inference(
            neg,
            &mut pos_w,
            &mut va,
            &mut |va, consequent, antecedents| {
                concrete_inferences.push(ConcreteInference {
                    consequent: consequent.concretize(va).unwrap(),
                    antecedents: antecedents
                        .iter()
                        .map(|antecedent| antecedent.concretize(va))
                        .collect::<Result<Vec<_>, ()>>()
                        .unwrap(),
                });
            },
        );
        concrete_inferences
    }
    fn big_step_inference(
        &self,
        neg: ComplementKnowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
        visit_inserted: &mut impl FnMut(&VariableAssignments, &RuleAtom, &Vec<RuleLiteral>),
    ) -> Knowledge {
        let mut pos_r = Knowledge::default(); // self.starting_facts();
        loop {
            for AnnotatedRule { v2d, rule } in &self.annotated_rules {
                rule.inference_stage(
                    v2d,
                    neg,
                    &pos_r,
                    pos_w,
                    va,
                    visit_inserted,
                    &self.executable_config,
                )
            }
            if pos_w.is_empty() {
                return pos_r;
            }
            pos_r.absorb_disjoint(pos_w);
        }
    }

    pub fn denotation(&self) -> DenotationResult {
        let mut pos_w = Knowledge::default(); // self.starting_facts();
        let mut va = VariableAssignments::default();
        let mut interpretations = vec![self.big_step_inference(
            ComplementKnowledge::Empty,
            &mut pos_w,
            &mut va,
            &mut |_, _, _| {},
        )];
        loop {
            if interpretations.len() % 2 == 1 {
                if let [.., a, b, c] = interpretations.as_mut_slice() {
                    if a == c {
                        // this is it!
                        use std::mem::take;
                        let [prev_truths, truths] = [take(b), take(c)];
                        let unknowns = {
                            let mut unknowns = prev_truths.clone();
                            for (did, set) in unknowns.map.iter_mut() {
                                set.retain(|atom| !truths.contains(did, atom))
                            }
                            unknowns
                        };
                        let emissions = Knowledge {
                            map: truths
                                .map
                                .iter()
                                .filter_map(|(did, set)| {
                                    if self.emissive.contains(did) {
                                        Some((did.clone(), set.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        };
                        let denotation = Denotation { truths, unknowns, emissions };
                        return DenotationResult { denotation, prev_truths };
                    }
                }
            }
            let neg = ComplementKnowledge::ComplementOf(interpretations.iter().last().unwrap());
            assert!(pos_w.map.is_empty());
            assert!(va.assignments.is_empty());
            let pos = self.big_step_inference(neg, &mut pos_w, &mut va, &mut |_, _, _| {});
            interpretations.push(pos);
        }
    }
}

impl Denotation<Knowledge> {
    pub fn bare(&self) -> Denotation<Bare<&Knowledge>> {
        let Self { truths, unknowns, emissions } = self;
        Denotation { truths: Bare(truths), unknowns: Bare(unknowns), emissions: Bare(emissions) }
    }
}

//////////////////////////////////////////////////////

impl ComplementKnowledge<'_> {
    fn contains(self, did: &DomainId, atom: &Atom) -> bool {
        match self {
            Self::Empty => false,
            Self::ComplementOf(k) => !k.contains(did, atom),
        }
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
    pub fn is_empty(&self) -> bool {
        self.map.values().all(HashSet::is_empty)
    }
    pub fn atoms_in_domain(&self, did: &DomainId) -> impl Iterator<Item = &Atom> + '_ {
        self.map.get(did).into_iter().flat_map(|set| set.iter())
    }
    pub fn contains(&self, did: &DomainId, atom: &Atom) -> bool {
        self.map.get(did).map(|set| set.contains(atom)).unwrap_or(false)
    }
    pub fn insert(&mut self, did: &DomainId, atom: Atom) -> bool {
        self.map.entry(did.clone()).or_default().insert(atom)
    }
    pub fn absorb_disjoint(&mut self, other: &mut Self) {
        for (did, set) in other.map.drain() {
            for atom in set {
                let new = self.insert(&did, atom);
                assert!(new);
            }
        }
    }
}

impl Atom {
    fn domain_id(&self) -> &DomainId {
        match self {
            Self::Constant { c } => c.domain_id(),
            Self::Construct { did, .. } => did,
        }
    }
    fn uniquely_assign_variables(
        &self,
        ra: &RuleAtom,
        va: &mut VariableAssignments,
    ) -> Result<(), ()> {
        match (self, ra) {
            (atom1, RuleAtom::Variable { vid, .. }) => va.insert(vid, atom1.clone()),
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
            (x, y) => panic!("bad concretisation {:?} given {:?}", (x, y), va),
        }
    }

    fn collect_subatoms<'a>(&'a self, set: &mut HashSet<&'a Self>) {
        if set.insert(self) {
            if let Self::Construct { args, .. } = self {
                for arg in args {
                    arg.collect_subatoms(set)
                }
            }
        }
    }
}

impl RuleAtom {
    fn concretize(&self, va: &VariableAssignments) -> Result<Atom, ()> {
        match self {
            RuleAtom::Variable { vid, .. } => va.get(vid).ok_or(()).cloned(),
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
        visit_inserted: &mut impl FnMut(&VariableAssignments, &RuleAtom, &Vec<RuleLiteral>),
        executable_config: &ExecutableConfig,
    ) {
        self.inference_stage_rec(
            v2d,
            neg,
            pos_r,
            pos_w,
            va,
            visit_inserted,
            &self.antecedents,
            executable_config,
        );
        va.assignments.clear();
    }

    fn inference_stage_rec(
        &self,
        v2d: &VariableTypes,
        neg: ComplementKnowledge,
        pos_r: &Knowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
        visit_inserted: &mut impl FnMut(&VariableAssignments, &RuleAtom, &Vec<RuleLiteral>),
        tail: &[RuleLiteral],
        executable_config: &ExecutableConfig,
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
                        let atom = consequent.concretize(va).expect("should work");
                        if executable_config.subconsequence {
                            let mut subatoms = HashSet::<&Atom>::default();
                            atom.collect_subatoms(&mut subatoms);
                            for subatom in subatoms {
                                let did = subatom.domain_id();
                                if !pos_r.contains(did, &subatom) {
                                    if pos_w.insert(did, subatom.clone()) {
                                        visit_inserted(va, consequent, &self.antecedents);
                                    }
                                }
                            }
                        } else {
                            let did = atom.domain_id();
                            if !pos_r.contains(did, &atom) {
                                if pos_w.insert(did, atom.clone()) {
                                    visit_inserted(va, consequent, &self.antecedents);
                                }
                            }
                        }
                    }
                }
            }
            [head, new_tail @ ..] => match head.sign {
                Sign::Pos => {
                    let did = head.ra.domain_id(v2d).expect("BAD");
                    for atom in pos_r.atoms_in_domain(&did) {
                        let state_token = va.get_state_token();
                        if atom.uniquely_assign_variables(&head.ra, va).is_ok() {
                            self.inference_stage_rec(
                                v2d,
                                neg,
                                pos_r,
                                pos_w,
                                va,
                                visit_inserted,
                                new_tail,
                                executable_config,
                            )
                        }
                        va.restore_state(state_token).expect("oh no");
                    }
                }
                Sign::Neg => self.inference_stage_rec(
                    v2d,
                    neg,
                    pos_r,
                    pos_w,
                    va,
                    visit_inserted,
                    new_tail,
                    executable_config,
                ),
            },
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
                did.fmt(f)?;
                if !args.is_empty() {
                    write!(f, "({:?})", crate::util::CommaSep { iter: args, spaced: false })?;
                }
                Ok(())
            }
        }
    }
}
impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Sign::Neg = self.sign {
            write!(f, "!")?
        }
        self.atom.fmt(f)
    }
}

impl std::fmt::Debug for Knowledge {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use crate::util::{map_snd, sorted_vec};
        let sorted_did_atoms =
            map_snd(self.map.iter().filter(|(_did, set)| !set.is_empty()), sorted_vec);
        f.debug_map().entries(sorted_vec(sorted_did_atoms)).finish()
    }
}

impl std::fmt::Debug for Bare<&Knowledge> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let truths = crate::util::sorted_vec(self.0.map.values().flat_map(HashSet::iter));
        f.debug_set().entries(truths).finish()
    }
}
