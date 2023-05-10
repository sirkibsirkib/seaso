use crate::ast::*;

struct AnonVariableAllocator {
    next: u32,
}

trait DeanonymizeVids {
    fn deanonymize_vids(&mut self, ava: &mut AnonVariableAllocator);
}

////////////////

/// Strips substrings that follow '#' but precede '\n' or the end of the string.
pub fn remove_line_comments(s: &mut String) {
    let mut outside_comment = true;
    s.retain(|c| {
        if c == '#' {
            outside_comment = false;
        } else if c == '\n' {
            outside_comment = true;
        }
        outside_comment
    })
}

/// Replaces each occurrence of `VariableId("_")` with a new identifier.
/// Assumes that no variables already in the program have prefix "v", which is the case for all parsed programs.
pub fn deanonymize_variable_ids(program: &mut Program) {
    // let mut ava = AnonVariableAllocator { next: 0 };
    program.deanonymize_vids(&mut AnonVariableAllocator { next: 0 })
}

impl DeanonymizeVids for Program {
    fn deanonymize_vids(&mut self, ava: &mut AnonVariableAllocator) {
        for statement in &mut self.statements {
            if let Statement::Rule(Rule { consequents, antecedents }) = statement {
                for ra in consequents.iter_mut().chain(antecedents.iter_mut().map(|rl| &mut rl.ra))
                {
                    ra.deanonymize_vids(ava)
                }
            }
        }
    }
}
impl DeanonymizeVids for RuleAtom {
    fn deanonymize_vids(&mut self, ava: &mut AnonVariableAllocator) {
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
                    arg.deanonymize_vids(ava)
                }
            }
        }
    }
}
