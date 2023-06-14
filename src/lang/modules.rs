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
pub struct ModuleSystem<'a> {
    step_toward: HashMap<[&'a ModuleName; 2], &'a ModuleName>,
}
impl<'a> ModuleSystem<'a> {
    pub fn new(modules: impl Iterator<Item = &'a Module>) -> Result<Self, ModuleSystemError<'a>> {
        let mut map: HashMap<&ModuleName, &Module> = Default::default();
        let mut modification: HashMap<(&ModuleName, &DomainId), &Statement> = Default::default();
        use std::collections::hash_map::Entry;
        for module in modules {
            let add = |did, statement| {
                modification.entry((&module.name, did)).or_insert(statement);
            };
            // for statement in &module.statements {
            //     match statement {
            //         Statement::Rule(Rule{ consequents, .. }) => {
            //             for consequent in consequents {
            //                 add(consequent.ra.did)
            //             }
            //         }
            //         Statement::Emit(did) => {
            //             // todo
            //         }
            //     }

            // }
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
        let mut step_toward: HashMap<[&'a ModuleName; 2], &'a ModuleName> = Default::default();
        // floyd warshall 2: given edges
        for module in map.values() {
            for dest in module.uses.elements() {
                if dest == &module.name {
                    continue;
                }
                step_toward.entry([&module.name, &dest]).or_insert(&dest);
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
                    if step_toward.contains_key(&[a, b]) && step_toward.contains_key(&[b, c]) {
                        step_toward.entry([a, c]).or_insert(b);
                    }
                }
            }
        }
        Ok(ModuleSystem { step_toward })
    }
}
