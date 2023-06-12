use std::collections::{HashMap, HashSet};

struct Domain(u16);
struct Const(u16);
struct VariableIdx(u16);

struct ConstSliceStore {
    data: Vec<Const>,
    atom_len: usize,
}

struct ConstSliceStoreIter<'a> {
    store: &'a ConstSliceStore,
    next: StoreConstIdx,
    zero_len_and_full: bool,
}

#[derive(PartialEq, Eq)]
struct StoreConstIdx(u16);

struct BufIdx(u16);

enum Instruction {
    FixedToBuffer(Const, BufIdx),
    BufferToStore { start: BufIdx, end: BufIdx, domain: Domain },
    BufferToBuffer { src_start: BufIdx, src_end: BufIdx, dest_start: BufIdx },
    TestBuffer { from: BufIdx, domain: Domain, contains: bool },
}

struct Rule {
    variable_domains: Vec<Domain>,
    instructions: Vec<Instruction>,
}

struct Program {
    domain_params: HashMap<Domain, Vec<Domain>>,
    domain_consts: HashMap<Domain, u16>,
    fixed: HashMap<Const, crate::ast::Constant>,
    facts: Store,
    rules: Vec<Rule>,
}
struct Store {
    map: HashMap<Domain, HashSet<StoreConstIdx>>,
}

////////////////

impl Program {
    fn infer(&self, r: &Store, w: &mut Store) -> bool {
        let mut modified_ever = false;
        loop {
            let mut modified_this_loop = false;
            for rule in &self.rules {}
            if !modified_this_loop {
                return modified_ever;
            }
        }
    }
}

impl<'a> Iterator for ConstSliceStoreIter<'a> {
    type Item = &'a [Const];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.store.data.len();
        if self.next.0 as usize >= len {
            if self.zero_len_and_full {
                self.zero_len_and_full = false;
                Some(&[])
            } else {
                None
            }
        } else {
            self.next.0 += self.store.atom_len as u16;
            Some(
                &self.store.data
                    [(self.next.0 as usize - self.store.atom_len)..self.next.0 as usize],
            )
        }
    }
}
