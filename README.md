# Seaso Executor

## Preparation
You will need:
1. `rustc`: the Rust language compiler `rustc` (https://www.rust-lang.org/). I am using `rustc` version 1.69, but many older versions will do fine. (If it complies, all is well).
2. `cargo`: the Rust package manager.
Both of these can be conveniently installed using `rustup`, the Rust package manager (https://rustup.rs/).

To acquire and compile from source:
1. Use `git` to clone this repo.
2. Working in the cloned directory, compile with `cargo build --release`.
The compiled binary ('the Seaso executor') will be found at `./target/release/seaso` (or `.\target\release\seaso.exe` on Windows).

## Usage
Run the Seaso executor, feeding in your source code as standard input. For example, run `./target/release/seaso_executor.exe < ./program_examples/toy1.seaso`.

## Output

Once the Seaso executor has consumed all standard input, it will parse and check if your Seaso program is well-formed, and if, so, will compute and output its denotation.
The denotation consists of three sets of atoms: _truths_, _unknowns_, and _emissions_.
Always, truths 

```
times taken:
- parse 53.5µs
- check 834µs
- denote 216.8µs
Denotation:
- trues: {x(8), x(9), y(8, 8), y(9, 8), y(9, 9), z(8, 9)}
- unknown: {}
- emissions: {}
seal broken: None
undeclared domains: {}
check result Ok(
    (),
)
parse result Ok(())
```

