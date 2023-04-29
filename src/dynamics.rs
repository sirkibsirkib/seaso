use crate::ast::*;
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

fn enumerate_rule(
    rule: &Rule,
    antecedents: &[RuleLiteral],
    vm: &mut VarMap,
    knowledge: &mut Knowledge,
) {
    match antecedents {
        [] => {
            // check checkable antecedents (TODO can be more efficient)

            // write consequents
            for consequent in &rule.consequents {
                let ra = consequent.concretized(vm).expect("WAH");
                let did = match &ra {
                    RuleAtom::Construct { did, .. } => did.clone(),
                    _ => unreachable!(),
                };
                knowledge.map.entry(did).or_default().insert(ra);
            }
        }
        [head, tail @ ..] => {
            todo!()
        }
    }
}
