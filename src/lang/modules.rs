const TEST: &str = "
    a,b,c!

    a:b,c,d
    @ \"dir1/dir2/filename.seaso\"
";

use std::collections::HashSet;

struct ModulePath {
    dirs: Vec<String>,
    filename: String,
}

struct ModuleName(String);

struct ModuleDef {
    name: ModuleName,
    needs: HashSet<HashSet<ModuleName>>,
    path: ModulePath,
}

enum SpecStatement {
    ModuleDef(ModuleDef),
    Includes(Vec<ModuleName>),
}
struct Spec {
    statements: Vec<SpecStatement>,
}

struct Edge<T> {
    from: T,
    to: T,
}
struct GraphEdges<T> {
    edges: HashSet<Edge<T>>,
}
