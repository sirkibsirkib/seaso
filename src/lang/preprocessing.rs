use crate::{statics::Module, *};
use std::collections::HashMap;

trait VisitMut<T> {
    fn visit_mut<F: FnMut(&mut T)>(&mut self, f: &mut F);
}

pub struct EqClasser {
    edges: Vec<[DomainId; 2]>,
}

#[derive(Debug)]
pub struct EquateIntStrErr {
    eq_class: HashSet<DomainId>,
}

////////////////
pub fn equate_domain_ids(modules: &mut [Module]) -> Result<EqClasses, EquateIntStrErr> {
    // step 1: extract equivalence classes
    let mut eq_classer = EqClasser { edges: vec![] };
    for module in modules.iter_mut() {
        for statement in module.statements.iter() {
            if let Statement::Decl(vec) = statement {
                for slice in vec.windows(2) {
                    if let [a, b] = slice {
                        eq_classer.add(a.clone(), b.clone());
                    } else {
                        unreachable!()
                    }
                }
            }
        }
    }
    let eq_classes = eq_classer.to_equivalence_classes()?;
    // step 2: rename dids
    for module in modules.iter_mut() {
        let mut guard = module.statements.as_vec_mut();
        for statement in guard.as_mut().iter_mut() {
            let mut clos = |did: &mut DomainId| {
                if let Some(representative) = eq_classes.get_representative(did) {
                    *did = representative.clone();
                }
            };
            statement.visit_mut(&mut clos);
        }
    }
    Ok(eq_classes)
}

pub fn deanonymize_variables(modules: &mut [Module]) {
    for module in modules {
        let mut guard = module.statements.as_vec_mut();
        for statement in guard.as_mut().iter_mut() {
            if let Statement::Rule(r) = statement {
                let mut next_idx = 0;
                let mut clos = |vid: &mut VariableId| {
                    if vid.0 == "_" {
                        *vid = VariableId(format!("V{}ANON", next_idx));
                        next_idx += 1;
                    }
                };
                r.visit_mut(&mut clos);
            }
        }
    }
}

impl VisitMut<DomainId> for RuleAtom {
    fn visit_mut<F: FnMut(&mut DomainId)>(&mut self, f: &mut F) {
        match self {
            Self::Variable(..) | Self::Constant(..) => {}
            Self::Construct { did, args } => {
                f(did);
                for arg in args {
                    arg.visit_mut(f)
                }
            }
        }
    }
}
impl VisitMut<DomainId> for Statement {
    fn visit_mut<F: FnMut(&mut DomainId)>(&mut self, f: &mut F) {
        match self {
            Self::Rule(rule) => {
                for ra in rule.root_atoms_mut() {
                    ra.visit_mut(f)
                }
            }
            Self::Defn { did, params } => {
                f(did);
                for param in params {
                    f(param)
                }
            }
            Self::Seal(did) | Self::Emit(did) => f(did),
            Self::Decl(dids) => {
                for did in dids {
                    f(did)
                }
            }
        }
    }
}

impl<T> VisitMut<T> for Rule
where
    RuleAtom: VisitMut<T>,
{
    fn visit_mut<F: FnMut(&mut T)>(&mut self, f: &mut F) {
        for ra in self.root_atoms_mut() {
            ra.visit_mut(f)
        }
    }
}

impl VisitMut<VariableId> for RuleAtom {
    fn visit_mut<F: FnMut(&mut VariableId)>(&mut self, f: &mut F) {
        match self {
            Self::Construct { args, .. } => {
                for arg in args {
                    arg.visit_mut(f)
                }
            }
            Self::Variable(vid) => f(vid),
            Self::Constant(..) => {}
        }
    }
}

/// Strips substrings that follow '#' but precede '\n' or the end of the string.
pub fn comments_removed(mut s: String) -> String {
    #[derive(Copy, Clone)]
    enum State {
        Outside,
        LineComment,
        BlockComment,
    }
    use State::*;
    let mut state = Outside;
    s.retain(|c| {
        let (new_state, retain) = match (state, c) {
            (Outside, '#') => (LineComment, false),
            (Outside, '<') => (BlockComment, false),
            (LineComment, '\n') => (Outside, true),
            (BlockComment, '>') => (Outside, false),
            (Outside, _) => (Outside, true),
            (s, _) => (
                s,
                match s {
                    Outside => true,
                    LineComment | BlockComment => false,
                },
            ),
        };
        state = new_state;
        retain
    });
    s
}

impl EqClasser {
    fn add(&mut self, a: DomainId, b: DomainId) {
        use core::cmp::Ordering;
        self.edges.push(match a.cmp(&b) {
            Ordering::Less => [a, b],
            Ordering::Greater => [b, a],
            Ordering::Equal => return,
        })
    }
    fn to_equivalence_classes(mut self) -> Result<EqClasses, EquateIntStrErr> {
        let mut representatives = HashMap::<DomainId, DomainId>::default();
        self.edges.sort();
        self.edges.dedup();
        for [a, b] in self.edges {
            // a < b
            let representative =
                representatives.get(&a).or(representatives.get(&b)).unwrap_or(&a).clone();
            representatives.insert(a, representative.clone());
            representatives.insert(b, representative);
        }
        let mut representative_members = HashMap::<DomainId, HashSet<DomainId>>::default();
        for (member, representative) in &representatives {
            representative_members
                .entry(representative.clone())
                .or_default()
                .insert(member.clone());
        }
        let mut eq_classes = EqClasses { representatives, representative_members };
        match [
            eq_classes.get_representative(DomainId::int()),
            eq_classes.get_representative(DomainId::str()),
        ] {
            [Some(a), Some(b)] if a == b => {
                let a = a.clone();
                return Err(EquateIntStrErr {
                    eq_class: eq_classes.representative_members.remove(&a).unwrap(),
                });
            }
            _ => {
                eq_classes.make_representative(DomainId::int());
                eq_classes.make_representative(DomainId::str());
                Ok(eq_classes)
            }
        }
    }
}
impl EqClasses {
    pub fn get_representative<'a, 'b>(&'a self, t: &'b DomainId) -> Option<&'a DomainId> {
        self.representatives.get(t)
    }
    pub fn get_representative_members(&self, t: &DomainId) -> Option<&HashSet<DomainId>> {
        self.representative_members.get(t)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&DomainId, &HashSet<DomainId>)> {
        self.representative_members.iter()
    }
    pub fn make_representative(&mut self, new: &DomainId) {
        if let Some(old) = self.representatives.get_mut(new) {
            if old == new {
                return; // nothing to do here
            }
            let old = std::mem::replace(old, new.clone());
            let class = self.representative_members.remove(&old).unwrap();
            for member in class.iter() {
                self.representatives.insert(member.clone(), new.clone());
            }
            self.representative_members.insert(new.clone(), class);
        } else {
            // nothing to do here
        }
    }
}
