# Seaso Executor
This contains the source code of the Seaso executor, which can be compiled to a standalone executable, or used as a Rust library.
Seaso is a simple logic programming language.
In a nutshell, each Seaso program (1) models the facts of a system, (2) prescribes which facts are undesirable, and (3) specifies how the program may be extended.
Seaso is based on the well-founded semantics by Van Gelder et al, and the underlying design decisions are motivated by its application to data exchange systems.  

## Executable

### Preparation
You will need:
1. `rustc`: the Rust language compiler `rustc` (https://www.rust-lang.org/). I am using `rustc` version 1.69, but many older versions will do fine. (If it complies, all is well).
2. `cargo`: the Rust package manager.
Both of these can be conveniently installed using `rustup`, the Rust installer and toolchain manager (https://rustup.rs/).

### Compilation
To acquire and compile from source:
1. Acquire a local copy of this repository (e.g. by using `git clone`).
2. Working in the cloned directory, compile with `cargo build --release`.
The compiled binary ('the Seaso executor') will be found at `./target/release/seaso` (or `.\target\release\seaso.exe` on Windows).

### Usage
Run the Seaso executor, feeding in your source code as standard input. For example, run the following:
```
./target/release/seaso.exe < ./program_examples/trust.seaso`
```

### Output

Once the Seaso executor has consumed all standard input, it will parse and check if your Seaso program is well-formed, and if, so, will compute and output its denotation.
The denotation consists of three sets of atoms: _truths_, _unknowns_, and _emissions_.
Always, truths and unknowns are disjoint, and truths are a superset of emissions.
Here is an example output resulting from the above usage.

```
dependend undefined parts: {}
used undeclared domains: {}
seal breaks: {}
denotation: Denotation {
    truths: {
        eq: [
            eq(party("Amy"), party("Amy")),
            eq(party("Bob"), party("Bob")),
            eq(party("Dan"), party("Dan")),
        ],
        party: [
            party("Amy"),
            party("Bob"),
            party("Dan"),
        ],
        trust: [
            trust(party("Amy"), party("Bob")),
            trust(party("Amy"), party("Dan")),
            trust(party("Dan"), party("Bob")),
        ],
        trusted: [
            trusted(party("Bob")),
            trusted(party("Dan")),
        ],
        untrusted: [
            untrusted(party("Amy")),
        ],
        very_trusted: [
            very_trusted(party("Bob")),
        ],
    },
    unknowns: {},
    emissions: {
        untrusted: [
            untrusted(party("Amy")),
        ],
    },
}
```

### CLI options
Run the tool with flag `--help` to see the optional arguments, used to customize the output.

Some of the arguments change the preprocessor, for example, only _without_ `--no-local` is the former program preprocessed to the latter:
```seaso
part x { decl   x. }
```
```seaso
part x { decl a@x. }
```

Most of the arguments change which metadata is printed. For exampe, _with_ `--ast1` and `--ast2`, the abstract syntax tree is printed before and after preprocessing, respectively.  

## Source and library

Source code documentation can be generated with `cargo doc --no-deps`, producing HTML documentation in `target\doc`.
For extra convenience, run with `cargo doc --no-deps --open` to open the docs in your default browser.
The repo can be used as a Rust dependency as usual (see `crates.io` for some examples).