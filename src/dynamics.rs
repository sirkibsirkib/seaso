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

#[derive(Default, PartialEq, Eq)]
pub struct Knowledge {
    map: HashMap<DomainId, HashSet<Atom>>,
}

#[derive(Debug)]
pub struct Denotation {
    pub pos: Knowledge,
    pub unk: Knowledge,
}

#[derive(Debug, Default)]
struct VariableAssignments {
    assignments: Vec<(VariableId, Atom)>,
}
struct StateToken {
    assignments_count: usize,
}

#[derive(Copy, Clone)]
enum NegativeKnowledge<'a> {
    Empty,
    ComplementOf(&'a Knowledge),
}

impl NegativeKnowledge<'_> {
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
        let iter = self.map.values().flat_map(HashSet::iter);
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
    fn big_step_inference(
        &self,
        r2v2d: &RuleToVidToDid,
        neg: NegativeKnowledge,
        pos_w: &mut Knowledge,
    ) -> Knowledge {
        assert!(pos_w.map.is_empty());
        let mut pos_r = Knowledge::default();
        loop {
            for (sidx, statement) in self.statements.iter().enumerate() {
                if let Statement::Rule(rule) = statement {
                    let v2d = r2v2d.get(&sidx).expect("wah");
                    rule.inference_stage(v2d, neg, &pos_r, pos_w);
                }
            }
            let changed = pos_r.absorb_all(pos_w);
            // println!("POS SO FAR {:?}", pos_r);
            if !changed {
                assert!(pos_w.map.is_empty());
                return pos_r;
            }
        }
    }

    pub fn denotation(&self, r2v2d: &RuleToVidToDid) -> Denotation {
        let mut pos_w = Knowledge::default();
        let mut interpretations =
            vec![self.big_step_inference(r2v2d, NegativeKnowledge::Empty, &mut pos_w)];
        loop {
            if interpretations.len() % 2 == 0 {
                if let [.., a, b, c, d] = interpretations.as_mut_slice() {
                    if a == c && b == d {
                        use std::mem::take;
                        let pos = take(d);
                        let mut unk = take(c);
                        for (did, set) in unk.map.iter_mut() {
                            set.retain(|atom| !pos.contains(did, atom))
                        }
                        return Denotation { pos, unk };
                    }
                }
            }
            let neg = NegativeKnowledge::ComplementOf(interpretations.iter().last().unwrap());
            let pos = self.big_step_inference(r2v2d, neg, &mut pos_w);
            interpretations.push(pos);
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
    fn absorb_all(&mut self, other: &mut Self) -> bool {
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
            (Atom::Constant { c: c1 }, RuleAtom::Constant { c: c2 }) => {
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
        neg: NegativeKnowledge,
        pos_r: &Knowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
        tail: &[RuleLiteral],
    ) {
        match tail {
            [] => {
                // perform all checks
                let checks_pass = self.antecedents.iter().all(|antecedent| {
                    let did = antecedent.ra.domain_id(v2d).expect("static checked");
                    if antecedent.is_enumerable_in(v2d).is_none() {
                        let atom = antecedent.ra.concretize(va).expect("should work");
                        match antecedent.sign {
                            Sign::Pos => todo!(),
                            Sign::Neg => {
                                let res = neg.contains(did, &atom);
                                dbg!(pos_r, atom, res);
                                res
                            }
                        }
                    } else {
                        true
                    }
                });
                if checks_pass {
                    // all checks passed
                    for consequent in self.consequents.iter() {
                        let did = consequent.domain_id(v2d).expect("static checked");
                        let atom = consequent.concretize(va).expect("should work");
                        pos_w.insert(did, atom);
                    }
                }
            }
            [head, new_tail @ ..] => match head.is_enumerable_in(v2d) {
                Some(did) => {
                    assert_eq!(head.sign, Sign::Pos);
                    for atom in pos_r.iter_did(did) {
                        // println!("ASSIGN_BEFORE {:?}", va);

                        let state_token = va.get_state_token();
                        if atom.uniquely_assign_variables(&head.ra, va).is_err() {
                            break;
                        }
                        // println!("AFTER {:?}", va);
                        self.inference_stage_rec(v2d, neg, pos_r, pos_w, va, new_tail);
                        va.restore_state(state_token).expect("oh no");
                    }
                }
                None => self.inference_stage_rec(v2d, neg, pos_r, pos_w, va, new_tail),
            },
        }
    }
    fn inference_stage(
        &self,
        v2d: &VidToDid,
        neg: NegativeKnowledge,
        pos_r: &Knowledge,
        pos_w: &mut Knowledge,
    ) {
        let mut va = VariableAssignments::default();
        // dbg!(self);
        self.inference_stage_rec(v2d, neg, pos_r, pos_w, &mut va, &self.antecedents)
    }
}
