use crate::ast::*;
use crate::statics::RuleToVidToDid;
use crate::statics::VidToDid;

use std::collections::HashMap;
use std::collections::HashSet;

/// Concrete counterpart to `RuleAtom` with no domain info
#[derive(Hash, Eq, PartialEq, Clone)]
enum Atom {
    Constant { c: Constant },
    Construct { did: DomainId, args: Vec<Atom> },
}

#[derive(Default)]
pub struct Knowledge {
    map: HashMap<DomainId, HashSet<Atom>>,
}

#[derive(Default)]
struct VariableAssignments {
    assignments: Vec<(VariableId, Atom)>,
}
struct StateToken {
    assignments_count: usize,
}

impl std::fmt::Debug for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constant { c } => match c {
                Constant::Int(c) => {
                    let mut f = f.debug_tuple("int");
                    f.field(c);
                    f
                }
                Constant::Str(c) => {
                    let mut f = f.debug_tuple("str");
                    f.field(c);
                    f
                }
            },
            Self::Construct { did, args } => {
                let mut f = f.debug_tuple(&did.0);
                for arg in args {
                    f.field(arg);
                }
                f
            }
        }
        .finish()
    }
}

impl std::fmt::Debug for Knowledge {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let iter =
            self.map.iter().flat_map(|(did, set)| set.iter().map(move |atom| (&did.0, atom)));
        f.debug_set().entries(iter).finish()
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

impl Program {
    pub fn big_step_inference(&self, r2v2d: &RuleToVidToDid, neg: &Knowledge) -> Knowledge {
        let mut pos = Knowledge::default();
        let mut changed;
        loop {
            changed = false;
            for (sidx, statement) in self.statements.iter().enumerate() {
                if let Statement::Rule(rule) = statement {
                    let v2d = r2v2d.get(&sidx).expect("wah");
                    rule.inference_stage(v2d, neg, &mut pos, &mut changed);
                }
            }
            if !changed {
                return pos;
            }
        }
    }
}

impl Knowledge {
    fn iter_did(&self, did: &DomainId) -> impl Iterator<Item = &Atom> + '_ {
        self.map.get(did).into_iter().flat_map(|set| set.iter())
    }
    fn contains(&self, did: &DomainId, atom: &Atom) -> bool {
        self.map.get(did).map(|set| set.contains(atom)).unwrap_or(false)
    }
    fn insert(&mut self, did: &DomainId, atom: Atom) -> bool {
        self.map.entry(did.clone()).or_default().insert(atom)
    }
}

impl Atom {
    fn uniquely_assign_variables(
        &self,
        ra: &RuleAtom,
        va: &mut VariableAssignments,
    ) -> Result<(), ()> {
        match (self, ra) {
            (atom1, RuleAtom::Variable { vid }) => va.insert(vid, atom1.clone()),
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
            (Atom::Constant { c: c1 }, RuleAtom::Constant { c: c2 }) if c1 == c2 => Ok(()),
            _ => todo!(),
        }
    }
}

impl RuleAtom {
    fn concretize(&self, va: &VariableAssignments) -> Result<Atom, ()> {
        match self {
            RuleAtom::Variable { vid } => va.get(vid).ok_or(()).cloned(),
            RuleAtom::Constant { c } => Ok(Atom::Constant { c: c.clone() }),
            RuleAtom::Construct { args, did } => Ok(Atom::Construct {
                args: args.iter().map(|ra| ra.concretize(va)).collect::<Result<Vec<_>, _>>()?,
                did: did.clone(),
            }),
        }
    }
}

impl Rule {
    fn inference_stage_rec(
        &self,
        v2d: &VidToDid,
        neg: &Knowledge,
        pos: &mut Knowledge,
        va: &mut VariableAssignments,
        changed: &mut bool,
        tail: &[RuleLiteral],
    ) {
        let state_token = va.get_state_token();
        match tail {
            [] => {
                // perform all checks
                for antecedent in self.antecedents.iter() {
                    let did = antecedent.ra.domain_id(v2d).expect("static checked");
                    if antecedent.is_enumerable_in(v2d).is_none() {
                        let atom = antecedent.ra.concretize(va).expect("should work");
                        match antecedent.sign {
                            Sign::Pos => todo!(),
                            Sign::Neg => {
                                if !neg.contains(did, &atom) {
                                    return;
                                }
                            }
                        }
                    }
                }
                // all checks passed
                for consequent in self.consequents.iter() {
                    let did = consequent.domain_id(v2d).expect("static checked");
                    let atom = consequent.concretize(va).expect("should work");
                    if pos.insert(did, atom) {
                        *changed = true;
                    }
                }
                // base
            }
            [head, new_tail @ ..] => {
                if let Some(did) = head.is_enumerable_in(v2d) {
                    assert_eq!(head.sign, Sign::Pos);
                    for atom in pos.iter_did(did) {
                        if atom.uniquely_assign_variables(&head.ra, va).is_err() {
                            va.restore_state(state_token).expect("waheyy");
                            return;
                        }
                    }
                }
                self.inference_stage_rec(v2d, neg, pos, va, changed, new_tail);
                va.restore_state(state_token).expect("oh no");
            }
        }
    }
    fn inference_stage(
        &self,
        v2d: &VidToDid,
        neg: &Knowledge,
        pos: &mut Knowledge,
        changed: &mut bool,
    ) {
        let mut va = VariableAssignments::default();
        self.inference_stage_rec(v2d, neg, pos, &mut va, changed, &self.antecedents)
    }
}
