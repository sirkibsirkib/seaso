/// Defines the abstract syntax of Seaso programs.
pub mod ast;

/// Methods for preprocessing programs at source level (removing line comments) and abstract syntax level (Allocating fresh names for each VariableId("_")).
pub mod preprocessing;

/// Parsers for types in `ast`, e.g., `Program`.
pub mod parse;

/// Statics of Seaso, implementing the checking of well-formedness of programs, and assignment of types to variables.
pub mod statics;

/// Dynamics of Seaso, implementing methods and defining types needed to compute the denotation of a checked program.`
pub mod dynamics;

pub mod search;

mod util;
