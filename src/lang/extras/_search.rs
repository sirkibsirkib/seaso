use crate::{
    dynamics::{Atom, Denotation, Executable},
    DomainId, ExecutableProgram, RuleAtom,
};

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Badness {
    rank: usize,
    count: usize,
}

type UnaryFact = Atom; // Construct with one param, contains no variables

#[derive(Clone, Copy)]

/// ascending order of badness
pub struct DomainBadnessOrder<'a>(pub &'a [DomainId]);

struct InnerQuery<'a> {
    user_query: DomainBadnessOrder<'a>,
    enum_construct_param: Vec<(DomainId, DomainId)>,
}

#[derive(Debug)]
pub struct Best {
    facts: Vec<UnaryFact>,
    badness: Option<Badness>,
}

//////////////////////////

impl Into<RuleAtom> for Atom {
    fn into(self) -> RuleAtom {
        match self {
            Atom::Constant { c } => RuleAtom::Constant(c),
            Atom::Construct { did, args } => {
                RuleAtom::Construct { did, args: args.into_iter().map(Into::into).collect() }
            }
        }
    }
}

impl Denotation {
    fn badness(&self, user_query: DomainBadnessOrder) -> Option<Badness> {
        for (rank, bad_domain) in user_query.0.iter().enumerate() {
            let map = self.truths.map.get(bad_domain);
            let count = match map {
                None => 0,
                Some(m) => m.len(),
            };
            if count > 0 {
                return Some(Badness { rank, count });
            }
        }
        None
    }

    fn all_addable_facts<'a>(
        &'a self,
        inner_query: &'a InnerQuery,
    ) -> impl Iterator<Item = UnaryFact> + 'a {
        inner_query.enum_construct_param.iter().flat_map(|(did, param)| {
            self.truths
                .atoms_in_domain(param)
                .map(|arg| Atom::Construct { did: did.clone(), args: vec![arg.clone()] })
                .filter(|atom| !self.truths.contains(did, atom))
        })
    }
}

impl ExecutableProgram {
    fn search_rec(&self, inner_query: &InnerQuery, stack: &mut Vec<UnaryFact>, best: &mut Best) {
        let denotation = (stack.as_slice(), self).denotation();
        let badness = denotation.badness(inner_query.user_query);
        match [&badness, &best.badness] {
            [_, None] => return,
            [None, Some(_)] => {
                best.badness = None;
                best.facts = stack.clone();
                return;
            }
            [Some(x), Some(y)] => {
                if x < y {
                    best.badness = badness;
                    best.facts = stack.clone();
                }
            }
        }
        let facts = denotation.all_addable_facts(inner_query);
        for fact in facts {
            stack.push(fact.clone());
            self.search_rec(inner_query, stack, best);
            stack.pop();
            if best.badness.is_none() {
                return;
            }
        }
    }
}

impl ExecutableProgram {
    fn sealed(&self, did: &DomainId) -> bool {
        self.sealers_modifiers.get(did).map(|dsm| !dsm.sealers.is_empty()).unwrap_or(false)
    }
}

impl ExecutableProgram {
    pub fn search(&self, user_query: DomainBadnessOrder) -> Best {
        let inner_query = self.innerize_query(user_query);
        let mut best = Best { facts: vec![], badness: self.denotation().badness(user_query) };
        let mut stack = vec![];
        self.search_rec(&inner_query, &mut stack, &mut best);
        best
    }
    fn innerize_query<'a, 'b>(&'a self, user_query: DomainBadnessOrder<'b>) -> InnerQuery<'b> {
        let enum_construct_param = user_query
            .0
            .iter()
            .filter_map(|did| {
                if self.sealed(did) {
                    return None;
                }
                if let Some(params) = self.dd.get(did) {
                    if let [param] = params.as_slice() {
                        Some((did.clone(), param.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        InnerQuery { user_query, enum_construct_param }
    }
}
