# Seaso Executor
This contains the source code of the Seaso executor, which can be compiled to a standalone executable, or used as a Rust library.

## Executable

### Preparation
You will need:
1. `rustc`: the Rust language compiler `rustc` (https://www.rust-lang.org/). I am using `rustc` version 1.69, but many older versions will do fine. (If it complies, all is well).
2. `cargo`: the Rust package manager.
Both of these can be conveniently installed using `rustup`, the Rust package manager (https://rustup.rs/).

### Compilation
To acquire and compile from source:
1. Use `git` to clone this repo.
2. Working in the cloned directory, compile with `cargo build --release`.
The compiled binary ('the Seaso executor') will be found at `./target/release/seaso` (or `.\target\release\seaso.exe` on Windows).

### Usage
Run the Seaso executor, feeding in your source code as standard input. For example, run `./target/release/seaso.exe < ./program_examples/trust.seaso`.

### Output

Once the Seaso executor has consumed all standard input, it will parse and check if your Seaso program is well-formed, and if, so, will compute and output its denotation.
The denotation consists of three sets of atoms: _truths_, _unknowns_, and _emissions_.
Always, truths and unknowns are disjoint, and truths are a superset of emissions.
Here is an example output resulting from the above usage.

```
TimesTaken {
    parse: 59.3µs,
    check: 494.1µs,
    denote: 796.8µs,
}
Denotation {
    trues: {
        eq: [
            eq(party("Amy"), party("Amy")),
            eq(party("Bob"), party("Bob")),
            eq(party("Dan"), party("Dan")),
        ],
        untrusted: [
            untrusted(party("Amy")),
        ],
        party: [
            party("Amy"),
            party("Bob"),
            party("Dan"),
        ],
        very_trusted: [
            very_trusted(party("Bob")),
        ],
        trusted: [
            trusted(party("Bob")),
            trusted(party("Dan")),
        ],
        trust: [
            trust(party("Amy"), party("Bob")),
            trust(party("Amy"), party("Dan")),
            trust(party("Dan"), party("Bob")),
        ],
    },
    unknowns: {},
    emissions: {
        untrusted: [
            untrusted(party("Amy")),
        ],
    },
}
undeclared domains: {}
seal broken: None
check error None
parse error None
```

## Source and library

Source code documentation can be generated with `cargo doc --no-deps`, producing HTML documentation in `target\doc`.
For extra convenience, run with `cargo doc --no-deps --open`.
The repo can be used as a rust dependency as usual (see `crates.io` for some examples).