use crate::{statics::StatementUsage, *};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum ModuleSystemError<'a> {
    DistinctModuleDefinition(&'a ModuleName, [&'a Module; 2]),
}

#[derive(Debug)]
pub(crate) struct ModuleSystem<'a> {
    pub(crate) map: HashMap<&'a ModuleName, &'a Module>,
    pub(crate) reaches: HashSet<[&'a ModuleName; 2]>,
}

impl<'a> StatementUsage<&'a ModuleName> for ModuleSystem<'a> {
    fn uses(&self, pair: [&'a ModuleName; 2]) -> bool {
        self.reaches.contains(&pair)
    }
}
impl<'a> ModuleSystem<'a> {
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
