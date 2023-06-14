// use crate::*;
use crate::*;
use std::collections::HashMap;
use std::collections::HashSet;

// struct ModulePath {
//     dirs: Vec<String>,
//     filename: String,
// }

// struct ModuleDef {
//     name: ModuleName,
//     // needs: HashSet<HashSet<ModuleName>>,
//     path: ModulePath,
// }

// enum SpecStatement {
//     ModuleDef(ModuleDef),
//     Includes(Vec<ModuleName>),
// }
// struct Spec {
//     statements: Vec<SpecStatement>,
// }

// struct Edge<T> {
//     from: T,
//     to: T,
// }
// struct GraphEdges<T> {
//     edges: HashSet<Edge<T>>,
// }

#[derive(Debug)]
pub enum ModuleSystemError<'a> {
    DistinctModuleDefinition(&'a ModuleName, [&'a Module; 2]),
}

// seal broken case:
// m1 is NOT an ancestor of m2
// m1 seals
// m2 modifies

#[derive(Debug)]
pub(crate) struct ModuleSystem<'a> {
    pub(crate) map: HashMap<&'a ModuleName, &'a Module>,
    pub(crate) reaches: HashSet<[&'a ModuleName; 2]>,
}
impl<'a> ModuleSystem<'a> {
    pub fn find_break(&self, sniffer: &BreakSniffer<&'a ModuleName>) -> Option<Break<&ModuleName>> {
        for (did, [sealers, modifiers]) in &sniffer.sealers_modifiers {
            for &sealer in sealers {
                for &modifier in modifiers {
                    if sealer != modifier && !self.reaches.contains(&[sealer, modifier]) {
                        return Some(Break { did: did.clone(), sealer, modifier });
                    }
                }
            }
        }
        None
    }

    pub fn new(modules: impl Iterator<Item = &'a Module>) -> Result<Self, ModuleSystemError<'a>> {
        let mut map: HashMap<&ModuleName, &Module> = Default::default();
        use std::collections::hash_map::Entry;
        for module in modules {
            match map.entry(&module.name) {
                Entry::Vacant(e) => drop(e.insert(module)),
                Entry::Occupied(e) => {
                    if *e.get() == module {
                        // harmless duplication
                    } else {
                        return Err(ModuleSystemError::DistinctModuleDefinition(
                            &module.name,
                            [module, *e.get()],
                        ));
                    }
                }
            }
        }

        // floyd warshall 1: self loops
        let mut reaches = HashSet::<[&'a ModuleName; 2]>::default();
        // floyd warshall 2: given edges
        for module in map.values() {
            for dest in module.uses.elements() {
                if dest == &module.name {
                    continue;
                }
                reaches.insert([&module.name, &dest]);
            }
        }
        // floyd warshall 3: all-to-all reachability
        for &a in map.keys() {
            for &b in map.keys() {
                if a == b {
                    continue;
                }
                for &c in map.keys() {
                    if a == c || b == c {
                        continue;
                    }
                    if reaches.contains(&[a, b]) && reaches.contains(&[b, c]) {
                        reaches.insert([a, c]);
                    }
                }
            }
        }
        Ok(ModuleSystem { map, reaches })
    }
}
