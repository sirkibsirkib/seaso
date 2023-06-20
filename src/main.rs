mod lang;

use lang::*;
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn timed<T>(func: impl FnOnce() -> T) -> (Duration, T) {
    let start = Instant::now();
    let res = func();
    (start.elapsed(), res)
}

fn main() -> Result<(), ()> {
    let source = stdin_to_string().expect("bad stdin");
    let source = preprocessing::comments_removed(source);
    println!("source after preprocessing: <<\n{}\n>>", &source);

    let mut parse_result = nom::combinator::all_consuming(parse::modules_and_statements)(&source);

    match &mut parse_result {
        Ok((_, modules)) => {
            println!("modules: {:#?}", modules);
            for module in modules.iter_mut() {
                preprocessing::NamesVariables::name_variables(module)
            }
            let module_map_result =
                util::collect_map_lossless(modules.iter().map(|module| (&module.name, module)));
            match module_map_result {
                Err(clashing_name) => {
                    println!("clashing module name: {:?}", clashing_name)
                }
                Ok(module_map) => {
                    let uumn =
                        statics::used_undefined_module_names(&module_map).collect::<HashSet<_>>();
                    println!("used_undefined_module: {:#?}", uumn);

                    let mp = ModulePreorder::new(&module_map);
                    let ep = ExecutableProgram::new(&module_map);
                    if let Ok(ep) = ep.as_ref() {
                        let seal_breaks = mp.iter_breaks(&ep).collect::<HashSet<_>>();
                        println!("seal_breaks: {:#?}", &seal_breaks);

                        let denotation = dynamics::Executable::denotation(ep);
                        println!("denotation: {:#?}", &denotation);
                    }
                }
            }
        }
        Err(e) => {
            println!("parse error: {:#?}", e);
        }
    }

    Ok(())
}
