# Seaso Executor
This contains the source code of the Seaso executor, which can be compiled to a standalone executable, or used as a Rust library.
Seaso is a simple logic programming language.
In a nutshell, each Seaso program prescribes
In a nutshell, each Seaso program (1) models the facts of a system, (2) isolates logical inconsistencies as unknown facts, (2) prescribes which subset of true facts are undesirable, and (3) specifies how the program may be extended.
Seaso's inference semantics implements the well-founded semantics [1], and the underlying design decisions are motivated by its application to data exchange systems.


## Executable

### Preparing the Rust Toolchain
You will need:
1. `rustc`: the Rust language compiler `rustc` (https://www.rust-lang.org/). I am using `rustc` version 1.69, but many older versions will do fine. (If it complies, all is well).
2. `cargo`: the Rust package manager.
Both of these can be conveniently installed using `rustup`, the Rust installer and toolchain manager (https://rustup.rs/).

### Compiling the Compiler
To acquire and compile the Seaso compiler from its Rust source:
1. Acquire a local copy of this repository (e.g. by using `git clone`).
2. Working in the cloned directory, compile with `cargo build --release`.
The compiled binary ('the Seaso executor') will be found at `./target/release/seaso` (or `.\target\release\seaso.exe` on Windows).

### Usage
Run the Seaso executor, feeding in your source code as standard input. For example, run the following:
```
`.\target\release\seaso.exe < .\example_programs\hello_world.seaso`
```

### Output

Once the Seaso executor has consumed all standard input, it will parse and check if your Seaso program is well-formed, and if so, will compute and output its denotation.
The denotation consists of three sets of atoms: _truths_, _unknowns_, and _emissions_.
Always, truths and unknowns are disjoint, and truths are a superset of emissions. 
Intuitively, unknowns "contain contradictory facts" and emissions "highlight important truths".

Here is an example output resulting from running `.\target\release\seaso.exe < .\example_programs\hello_world.seaso`

```
denotation: Denotation {
    truths: {
        "Hello, world!",
        hello("Hello, world!"),
    },
    unknowns: {},
    emissions: {
        hello("Hello, world!"),
    },
}
```

### CLI options
Run the tool with flag `--help` to see the optional arguments, used to customize the output.

Some of the arguments change the preprocessor. For example, only with `--local` is program `part x { decl a. }` preprocessed to `part x { decl a@x. }`.

Most of the arguments change which metadata is printed. For example, _with_ `--ast1` and `--ast2`, the abstract syntax tree is printed before and after preprocessing, respectively.  

## Source and library

Source code documentation can be generated with `cargo doc --no-deps`, producing HTML documentation in `target\doc`.
For extra convenience, run with `cargo doc --no-deps --open` to open the docs in your default browser.
The repo can be used as a Rust dependency as usual (see `crates.io` for some examples).
For example, you can build a software system that uses this repo to compute the denotation of Seaso programs stored in memory as ASTs (skipping parsing).

## Language

The Seaso language is being developed for the incremental modelling of complex, federated, data-exchange systems. Once ready, the associated paper will be referred to here for a complete language definition. In the meantime, inspect `./example_programs/features` for simple Seaso programs chosen to illustrate language features.


## Executability of Examples Test

Compile the library, run all tests, and show which pass/fail with the following:
```
cargo test --release -- --nocapture
```
Note that this currently only tests whether the program was executable, i.e.,
undeclared domains and broken seals do not prevent passing.

If working as intended, you should see output beginning with the following:
```
running 1 test
pass ./example_programs\features\ascription.seaso
pass ./example_programs\features\asp_eg.seaso
pass ./example_programs\fearunning 1 test
pass ./example_programs\features_by_example\001_no_arguments.seaso
pass ./example_programs\features_by_example\002_defn_relations.seaso
pass ./example_programs\features_by_example\003_infer.seaso
pass ./example_programs\features_by_example\004_conjunct_consequents.seaso
pass ./example_programs\features_by_example\005_declare.seaso
pass ./example_programs\features_by_example\006_ascription.seaso
pass ./example_programs\features_by_example\007_negation.seaso
pass ./example_programs\features_by_example\008_types_vs_relations.seaso
pass ./example_programs\features_by_example\009_emit.seaso
pass ./example_programs\features_by_example\010_seal.seaso
pass ./example_programs\features_by_example\011_parts_namespaces.seaso
pass ./example_programs\features_by_example\012_parts_sealing.seaso
pass ./example_programs\features_by_example\013_unifying_types.seaso
pass ./example_programs\features_by_example\014_unifying_between_parts.seaso
pass ./example_programs\hello_world.seaso
pass ./example_programs\misc\brane.seaso
pass ./example_programs\misc\brane2.seaso
pass ./example_programs\misc\brane3.seaso
pass ./example_programs\misc\data_exchange.seaso
pass ./example_programs\misc\eg.seaso
pass ./example_programs\misc\integers.seaso
pass ./example_programs\misc\integers_simple.seaso
pass ./example_programs\misc\planning.seaso
pass ./example_programs\misc\redundant.seaso
pass ./example_programs\misc\simple_plan.seaso
pass ./example_programs\misc\trust.seaso
pass ./example_programs\misc\trust_badge.seaso
pass ./example_programs\misc\unl.seaso
pass ./example_programs\misc\unl2.seaso
pass ./example_programs\misc\violates_task.seaso
pass ./example_programs\paper\section3.seaso
pass ./example_programs\paper\section4.seaso
pass ./example_programs\paper\section5.seaso
test tests::examples ... ok
```

## References

[1] Van Gelder, Allen, Kenneth A. Ross, and John S. Schlipf. "The well-founded semantics for general logic programs." Journal of the ACM (JACM) 38.3 (1991): 619-649.