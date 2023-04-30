use crate::ast::*;
use crate::statics::RuleToVidToDid;
use crate::statics::VidToDid;

use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Eq, PartialEq, Debug, Clone)]
enum Atom {
    Constant { c: Constant },
    Construct { args: Vec<Atom> },
}

#[derive(Debug, Default)]
struct Knowledge {
    map: HashMap<DomainId, HashSet<Atom>>,
}

impl Program {
    fn big_step_inference(&self, r2v2d: &RuleToVidToDid, neg: &Knowledge) -> Knowledge {
        let mut pos = Knowledge::default();
        let mut changed;
        loop {
            changed = false;
            for (sidx, statement) in self.statements.iter().enumerate() {
                if let Statement::Rule(rule) = statement {
                    let v2d = r2v2d.get(&sidx).expect("wah");
                    changed |= rule.inference_stage(v2d, neg, &mut pos);
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
}

impl Atom {
    fn uniquely_assign_variables(
        &self,
        ra: &RuleAtom,
        v2a: &mut HashMap<VariableId, Atom>,
    ) -> Result<(), ()> {
        match (self, ra) {
            (atom1, RuleAtom::Variable { vid }) => match v2a.insert(vid.clone(), atom1.clone()) {
                Some(atom2) if atom1 != &atom2 => Err(()),
                _ => Ok(()),
            },
            (Atom::Construct { args: atoms }, RuleAtom::Construct { args: rule_atoms, .. }) => {
                if atoms.len() == rule_atoms.len() {
                    for (atom, rule_atom) in atoms.iter().zip(rule_atoms) {
                        atom.uniquely_assign_variables(rule_atom, v2a)?
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

impl Rule {
    fn inference_stage_rec(
        &self,
        v2d: &VidToDid,
        neg: &Knowledge,
        pos: &mut Knowledge,
        v2a: &mut HashMap<VariableId, Atom>,
        changed: &mut bool,
        tail: &[RuleLiteral],
    ) {
        match tail {
            [] => {
                // for
                // base
            }
            [head, new_tail @ ..] => match head.is_enumerable_in(v2d) {
                Some(did) => {
                    assert_eq!(head.sign, Sign::Pos);
                    for atom in pos.iter_did(did) {
                        atom.uniquely_assign_variables(&head.ra, v2a);
                    }
                }
                None => self.inference_stage_rec(v2d, neg, pos, v2a, changed, new_tail),
            },
        }
    }
    fn inference_stage(&self, v2d: &VidToDid, neg: &Knowledge, pos: &mut Knowledge) -> bool {
        let mut changed = false;
        let mut v2a = HashMap::<VariableId, Atom>::default();
        self.inference_stage_rec(v2d, neg, pos, &mut v2a, &mut changed, &self.antecedents);
        changed
    }
}

// struct Const {
//     Str(Str),
//     Int(i64),
// }

// struct State {
//     const_index: Indexer<Const>,
//     domain_index: Indexer<DomainId>,
// }

// struct EqCheck {
//     start_a: u8,
//     start_b: u8,
//     len: u8,
// }

// enum RuleInstruction {
//     Load
// }

// struct FalseCheck {

// }

// type ConstIndex = Index;
// type DomainIndex = Index;

// struct Rule {
//     enumerate: Vec<DomainIndex>,
//     eq_check: Vec<EqCheck>
// }

// struct ConstId(u16);

// struct ConstStore {
//     cid_to_const: HashMap<ConstId, Const>,
//     const_to_cid: HashMap<ConstId, Const>,
// }

// ///////////////

// struct AntecedentStates<'a> {
//     rule: &'a Rule,
//     s: Vec<Option<std::collections::hash_set::Iter<'a, RuleLiteral>>>,
// }

// impl<'a> AntecedentStates<'a> {
//     fn new(rule: &Rule) -> Self {
//         for antecedent in &rule.antecedents {
//             // /wah
//         }
//         todo!()
//     }
// }

// struct Knowledge {
//     map: HashMap<DomainId, HashSet<RuleAtom>>,
// }

// struct VarMap {
//     map: HashMap<VariableId, RuleAtom>,
// }

// #[derive(Debug)]
// enum ConcretizeAtomErr {
//     UnmappedVariable { vid: VariableId },
// }

// impl RuleAtom {
//     fn concretized(&self, vm: &VarMap) -> Result<RuleAtom, ConcretizeAtomErr> {
//         Ok(match self {
//             RuleAtom::Variable { vid } => vm
//                 .map
//                 .get(vid)
//                 .ok_or(ConcretizeAtomErr::UnmappedVariable { vid: vid.clone() })?
//                 .clone(),
//             RuleAtom::IntConst { .. } | RuleAtom::StrConst { .. } => self.clone(),
//             RuleAtom::Construct { did, args } => RuleAtom::Construct {
//                 did: did.clone(),
//                 args: args.iter().map(|ra| ra.concretized(vm)).collect::<Result<Vec<_>, _>>()?,
//             },
//         })
//     }
// }
// impl RuleAtom {
//     fn construct_did(&self) -> Option<&DomainId> {
//         match self {
//             RuleAtom::Construct { did, .. } => Some(did),
//             _ => None,
//         }
//     }
// }

// fn enumerate_rule(
//     rule: &Rule,
//     v2d: &VidToDid,
//     tail_antecedents: &[RuleLiteral],
//     vm: &mut VarMap,
//     pos_knowledge: &mut Knowledge,
//     neg_knowledge: &Knowledge,
// ) {
//     match tail_antecedents {
//         [] => {
//             // check checkable antecedents (TODO can be more efficient)
//             for RuleLiteral { sign, ra } in &rule.antecedents {
//                 let knowledge = match sign {
//                     Sign::Pos => pos_knowledge,
//                     Sign::Neg => neg_knowledge,
//                 };
//                 let did = ra.construct_did().expect("woo");
//                 if !knowledge
//                     .map
//                     .get(did)
//                     .map(|set| set.contains(&ra.concretized(vm).expect("WAH")))
//                     .unwrap_or(false)
//                 {
//                     // check failed!
//                     return;
//                 }
//             }
//             // write consequents
//             for consequent in &rule.consequents {
//                 let ra = consequent.concretized(vm).expect("WAH");
//                 let did = ra.construct_did().expect("woo").clone();
//                 pos_knowledge.map.entry(did).or_default().insert(ra);
//             }
//         }
//         [rl, tail @ ..] => {
//             let mut variables = HashSet::<VariableId>::default();
//             if let Some(did) = rl.is_enumerable_in(v2d) {
//                 for ra in pos_knowledge.map.entry(did).or_default().iter() {
//                     todo!()
//                 }
//             }
//         }
//     }
// }
