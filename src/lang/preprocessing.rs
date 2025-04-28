use crate::lang::VecSet;
use crate::*;
use core::cmp::Ordering;
use std::collections::{HashMap, HashSet};

trait VisitMut<T> {
    fn visit_mut<F: FnMut(&mut T)>(&mut self, f: &mut F);
}

pub struct EqDomainIdGraph<'a> {
    sorted_dir_edges: VecSet<[&'a DomainId; 2]>,
}

#[derive(Debug)]
pub struct EquatePrimitivesError<'a> {
    pub eq_class: &'a Vec<DomainId>,
}

////////////////

pub fn add_antecedent_variables_as_pos_literals(program: &mut Program) {
    let mut guard = program.parts.as_vec_mut();
    for part in guard.as_mut() {
        let mut guard = part.statements.as_vec_mut();
        for statement in guard.as_mut() {
            if let Statement::Rule(rule) = statement {
                let consequent_vids = rule.consequent_variables();
                for vid in consequent_vids {
                    if !rule.is_enumerable_variable(&vid) {
                        rule.antecedents.push(RuleLiteral {
                            ra: RuleAtom::Variable { vid, ascription: None },
                            sign: Sign::Pos,
                        });
                    }
                }
            }
        }
    }
}

pub fn normalize_domain_id_formatting(program: &mut Program, localize: bool) {
    let mut guard = program.parts.as_vec_mut();
    for part in guard.as_mut() {
        let mut guard = part.statements.as_vec_mut();
        for statement in guard.as_mut() {
            let mut clos = |did: &mut DomainId| {
                if did.is_primitive() {
                    return;
                }
                // do this regardless of `localize`
                did.0.retain(|c| !c.is_whitespace());
                // do this only if `localize`
                if localize && part.name.0 != "" && !did.0.contains("@") {
                    did.0.push_str("@");
                    did.0.push_str(&part.name.0);
                }
            };
            statement.visit_mut(&mut clos);
        }
    }
}

impl Default for EqDomainIdGraph<'_> {
    fn default() -> Self {
        Self { sorted_dir_edges: Default::default() }
    }
}
impl<'a> EqDomainIdGraph<'a> {
    fn insert(&mut self, a: &'a DomainId, b: &'a DomainId) {
        if a != b {
            self.sorted_dir_edges.insert([a, b]);
            self.sorted_dir_edges.insert([b, a]);
        }
    }
    fn out_edges(&'a self, did: &'a DomainId) -> impl Iterator<Item = &'a DomainId> + 'a {
        self.sorted_dir_edges.iter().filter(move |[from, _]| *from == did).map(|[_from, to]| *to)
    }
    fn dfs(&self, at: &DomainId, unmarked: &mut HashSet<&DomainId>, cluster: &mut Vec<DomainId>) {
        if unmarked.remove(at) {
            cluster.push(at.clone());
            for next in self.out_edges(at) {
                self.dfs(next, unmarked, cluster)
            }
        }
    }
    fn to_equivalence_classes(self) -> EqClasses {
        // collect all verts
        let mut unmarked = HashSet::default();
        for [a, b] in self.sorted_dir_edges.iter() {
            unmarked.insert(*a);
            unmarked.insert(*b);
        }

        // compute member -> representative
        let mut representative_members = HashMap::<DomainId, Vec<DomainId>>::default();
        while let Some(did) = unmarked.iter().next() {
            let mut cluster = vec![];
            self.dfs(did, &mut unmarked, &mut cluster);
            fn cmp(a: &DomainId, b: &DomainId) -> Ordering {
                match [a.is_primitive(), b.is_primitive()] {
                    [true, false] => Ordering::Less,
                    [false, true] => Ordering::Greater,
                    _ => a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(&b.0)),
                }
            }
            cluster.sort_by(cmp);
            let representative = cluster.first().unwrap().clone();
            representative_members.insert(representative, cluster);
        }
        // compute representative -> {member}
        let representatives: HashMap<DomainId, DomainId> = representative_members
            .iter()
            .flat_map(|(representative, members)| {
                members.iter().map(move |member| (member.clone(), representative.clone()))
            })
            .collect();

        // done!
        EqClasses { representatives, representative_members }
    }
}

pub fn deanonymize_variables(program: &mut Program) {
    let mut guard = program.parts.as_vec_mut();
    for part in guard.as_mut() {
        let mut guard = part.statements.as_vec_mut();
        for statement in guard.as_mut() {
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
            Self::Variable { .. } | Self::Constant(..) => {}
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
            Self::Variable { vid, .. } => f(vid),
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

impl EqClasses {
    pub fn new(program: &Program) -> Self {
        let mut graph = EqDomainIdGraph::default();
        let iter = program
            .parts
            .iter()
            .flat_map(|part| part.statements.iter())
            .chain(program.anon_mod_statements.iter());
        for statement in iter {
            if let Statement::Decl(vec) = statement {
                for slice in vec.windows(2) {
                    if let [a, b] = slice {
                        graph.insert(a, b);
                    } else {
                        unreachable!()
                    }
                }
            }
        }
        graph.to_equivalence_classes()
    }
    pub fn normalize_equal_domain_ids_in(&self, statement: &mut Statement) {
        let mut clos = |did: &mut DomainId| {
            if let Some(representative) = self.get_representative(did) {
                if representative != did {
                    *did = representative.clone();
                }
            }
        };
        statement.visit_mut(&mut clos);
    }
    pub fn normalize_equal_domain_ids(&self, program: &mut Program) {
        let mut guard = program.parts.as_vec_mut();
        for part in guard.as_mut() {
            let mut guard = part.statements.as_vec_mut();
            for statement in guard.as_mut() {
                self.normalize_equal_domain_ids_in(statement);
            }
        }
        for statement in program.anon_mod_statements.iter_mut() {
            self.normalize_equal_domain_ids_in(statement);
        }
    }
    pub fn get_representative<'a, 'b>(&'a self, t: &'b DomainId) -> Option<&'a DomainId> {
        self.representatives.get(t)
    }
    pub fn get_representatives(&self) -> &HashMap<DomainId, DomainId> {
        &self.representatives
    }
    pub fn get_representative_members(&self) -> &HashMap<DomainId, Vec<DomainId>> {
        &self.representative_members
    }
    pub fn check_primitives(&self) -> Result<(), EquatePrimitivesError> {
        for did in [DomainId::str(), DomainId::int()] {
            match self.get_representative(did) {
                Some(representative) if representative != did && representative.is_primitive() => {
                    return Err(EquatePrimitivesError {
                        eq_class: self.representative_members.get(representative).unwrap(),
                    })
                }
                _ => {}
            }
        }
        Ok(())
    }
}
