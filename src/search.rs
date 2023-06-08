use crate::{
    ast::{DomainId, RuleAtom},
    dynamics::{
        Atom, ComplementKnowledge, Denotation, Denotes, Knowledge, TakesBigSteps,
        VariableAssignments,
    },
    statics::{Checked, HasEmittedDomains},
};
use std::collections::HashSet;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Badness {
    rank: usize,
    count: usize,
}

#[derive(Clone)]
pub struct UnaryFact {
    pub did: DomainId,
    pub arg: Atom,
}

#[derive(Clone, Copy)]
pub struct UserQuery<'a>(&'a [DomainId]);
struct InnerQuery<'a> {
    user_query: UserQuery<'a>,
    enum_construct_param: Vec<(DomainId, DomainId)>,
}

struct ProgramWithFacts<'a> {
    checked: &'a Checked<'a>,
    facts: &'a [UnaryFact],
}

impl HasEmittedDomains for ProgramWithFacts<'_> {
    fn emitted_domains(&self) -> HashSet<&DomainId> {
        self.checked.emitted_domains()
    }
}
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
    fn badness(&self, user_query: UserQuery) -> Option<Badness> {
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
                .map(|arg| UnaryFact { did: did.clone(), arg: arg.clone() })
                .filter(|unary_fact| {
                    let atom = Atom::Construct {
                        did: unary_fact.did.clone(),
                        args: vec![unary_fact.arg.clone()],
                    };
                    !self.truths.contains(did, &atom)
                })
        })
    }
}

pub struct Best {
    facts: Vec<UnaryFact>,
    badness: Option<Badness>,
}

impl Checked<'_> {
    fn search_rec(&self, inner_query: &InnerQuery, stack: &mut Vec<UnaryFact>, best: &mut Best) {
        let denotation = ProgramWithFacts { checked: self, facts: stack }.denotation();
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
            stack.push(fact);
            self.search_rec(inner_query, stack, best);
            stack.pop();
            if best.badness.is_none() {
                return;
            }
        }
    }
}

impl TakesBigSteps for ProgramWithFacts<'_> {
    fn big_step_inference(
        &self,
        neg: ComplementKnowledge,
        pos_w: &mut Knowledge,
        va: &mut VariableAssignments,
    ) -> Knowledge {
        for UnaryFact { did, arg } in self.facts {
            pos_w.insert(did, Atom::Construct { did: did.clone(), args: vec![arg.clone()] });
        }
        self.checked.big_step_inference(neg, pos_w, va)
    }
}

impl Checked<'_> {
    pub fn search(&self, user_query: UserQuery) -> Best {
        let inner_query = self.innerize_query(user_query);
        let mut best = Best { facts: vec![], badness: self.denotation().badness(user_query) };
        let mut stack = vec![];
        self.search_rec(&inner_query, &mut stack, &mut best);
        best
    }
    fn innerize_query<'a, 'b>(&'a self, user_query: UserQuery<'b>) -> InnerQuery<'b> {
        let enum_construct_param = user_query
            .0
            .iter()
            .filter_map(|did| {
                if let Some((_statement_idx, params)) = self.dd.get(did) {
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
