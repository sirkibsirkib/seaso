use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Eq, Hash, PartialEq, Clone)]
struct ValId(u16);
#[derive(Eq, Hash, PartialEq, Clone)]
enum Val {
    Str(String),
    Int(i64),
}

struct ValMap {
    id_to_val: HashMap<ValId, Val>,
    val_to_id: HashMap<Val, ValId>,
    via: ValIdAllocator,
}

struct ValIdAllocator {
    next: ValId,
}

#[derive(Debug)]
struct NoFreeValIds;
///////////////////////
impl Default for ValMap {
    fn default() -> Self {
        ValMap {
            id_to_val: Default::default(),
            val_to_id: Default::default(),
            via: ValIdAllocator { next: ValId(0) },
        }
    }
}
impl ValMap {
    fn add(&mut self, val: Val) -> Result<ValId, NoFreeValIds> {
        Ok(if let Some(id) = self.val_to_id.get(&val) {
            id.clone()
        } else {
            let id = self.via.alloc()?;
            self.id_to_val.insert(id.clone(), val.clone());
            self.val_to_id.insert(val, id.clone());
            id
        })
    }
}

impl ValIdAllocator {
    fn alloc(&mut self) -> Result<ValId, NoFreeValIds> {
        let next_next = ValId(self.next.0.checked_add(1).ok_or(NoFreeValIds)?);
        Ok(std::mem::replace(&mut self.next, next_next))
    }
}
