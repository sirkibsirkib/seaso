mod lang;

use lang::*;
use std::collections::HashSet;

fn stdin_to_string() -> Result<String, std::io::Error> {
    use std::io::Read as _;
    let mut buffer = String::new();
    std::io::stdin().lock().read_to_string(&mut buffer)?;
    Ok(buffer)
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
            let module_map_result = statics::ModuleMap::new(modules.iter());
            match module_map_result {
                Err(clashing_name) => println!("clashing module name: {:?}", clashing_name),
                Ok(module_map) => {
                    let uumn = module_map.used_undefined_names().collect::<HashSet<_>>();
                    println!("used_undefined_module: {:#?}", uumn);
                    let ep = ExecutableProgram::new(&module_map);
                    match ep {
                        Ok(ep) => {
                            println!("used undeclared: {:?}", &ep.used_undeclared);
                            let mp = statics::ModulePreorder::new(&module_map);
                            let seal_breaks = mp.iter_breaks(&ep).collect::<HashSet<_>>();
                            println!("seal breaks: {:#?}", &seal_breaks);
                            println!("asp print:\n{}", ep.asp_print().expect("ok"));
                            println!("ontology dot:\n{}", ep.ontology_dot().expect("should"));
                            let denotation = dynamics::Executable::denotation(&ep);
                            println!("denotation: {:#?}", &denotation);
                        }
                        Err(e) => println!("executable error: {:#?}", e),
                    }
                }
            }
        }
        Err(e) => println!("parse error: {:#?}", e),
    }
    Ok(())
}
