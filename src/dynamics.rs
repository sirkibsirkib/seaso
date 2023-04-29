use crate::ast::*;
use crate::statics::VidToDid;
use std::collections::HashMap;
use std::collections::HashSet;

struct AntecedentStates<'a> {
    rule: &'a Rule,
    s: Vec<Option<std::collections::hash_set::Iter<'a, RuleLiteral>>>,
}

impl<'a> AntecedentStates<'a> {
    fn new(rule: &Rule) -> Self {
        for antecedent in &rule.antecedents {
            // /wah
        }
        todo!()
    }
}

struct Knowledge {
    map: HashMap<DomainId, HashSet<RuleAtom>>,
}

struct VarMap {
    map: HashMap<VariableId, RuleAtom>,
}

#[derive(Debug)]
enum ConcretizeAtomErr {
    UnmappedVariable { vid: VariableId },
}

impl RuleAtom {
    fn concretized(&self, vm: &VarMap) -> Result<RuleAtom, ConcretizeAtomErr> {
        Ok(match self {
            RuleAtom::Variable { vid } => vm
                .map
                .get(vid)
                .ok_or(ConcretizeAtomErr::UnmappedVariable { vid: vid.clone() })?
                .clone(),
            RuleAtom::IntConst { .. } | RuleAtom::StrConst { .. } => self.clone(),
            RuleAtom::Construct { did, args } => RuleAtom::Construct {
                did: did.clone(),
                args: args.iter().map(|ra| ra.concretized(vm)).collect::<Result<Vec<_>, _>>()?,
            },
        })
    }
}
impl RuleAtom {
    fn construct_did(&self) -> Option<&DomainId> {
        match self {
            RuleAtom::Construct { did, .. } => Some(did),
            _ => None,
        }
    }
}

fn enumerate_rule(
    rule: &Rule,
    v2d: &VidToDid,
    tail_antecedents: &[RuleLiteral],
    vm: &mut VarMap,
    pos_knowledge: &mut Knowledge,
    neg_knowledge: &Knowledge,
) {
    match tail_antecedents {
        [] => {
            // check checkable antecedents (TODO can be more efficient)
            for RuleLiteral { sign, ra } in &rule.antecedents {
                let knowledge = match sign {
                    Sign::Pos => pos_knowledge,
                    Sign::Neg => neg_knowledge,
                };
                let did = ra.construct_did().expect("woo");
                if !knowledge
                    .map
                    .get(did)
                    .map(|set| set.contains(&ra.concretized(vm).expect("WAH")))
                    .unwrap_or(false)
                {
                    // check failed!
                    return;
                }
            }
            // write consequents
            for consequent in &rule.consequents {
                let ra = consequent.concretized(vm).expect("WAH");
                let did = ra.construct_did().expect("woo").clone();
                pos_knowledge.map.entry(did).or_default().insert(ra);
            }
        }
        [rl, tail @ ..] => {
            let mut variables = HashSet::<VariableId>::default();
            if let Some(did) = rl.is_enumerable_in(v2d) {
                for ra in pos_knowledge.map.entry(did).or_default().iter() {
                    todo!()
                }
            }
        }
    }
}
