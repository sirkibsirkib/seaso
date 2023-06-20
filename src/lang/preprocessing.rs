use crate::*;

pub struct AnonVariableAllocator {
    next: u32,
}

pub trait NamesVariables {
    fn name_variables(&mut self);
}

trait NamesVariablesWithContext {
    fn name_variables(&mut self, ava: &mut AnonVariableAllocator);
}

////////////////

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

impl NamesVariables for Module {
    fn name_variables(&mut self) {
        for s in self.statements.as_slice_mut().as_mut() {
            s.name_variables(&mut AnonVariableAllocator { next: 0 });
        }
    }
}

impl NamesVariablesWithContext for Statement {
    fn name_variables(&mut self, ava: &mut AnonVariableAllocator) {
        if let Statement::Rule(Rule { antecedents, .. }) = self {
            // Only deanonymize ENUMERABLE variables!
            for antecedent in antecedents {
                if antecedent.sign == Sign::Pos {
                    antecedent.ra.name_variables(ava)
                }
            }
        }
    }
}
impl NamesVariablesWithContext for RuleAtom {
    fn name_variables(&mut self, ava: &mut AnonVariableAllocator) {
        match self {
            RuleAtom::Variable(vid) => {
                if &vid.0 == "_" {
                    vid.0 = format!("v{}", ava.next);
                    ava.next += 1
                }
            }
            RuleAtom::Constant(_) => {}
            RuleAtom::Construct { args, .. } => {
                for arg in args {
                    arg.name_variables(ava)
                }
            }
        }
    }
}
