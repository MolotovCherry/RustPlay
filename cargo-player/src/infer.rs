use crate::File;

use syn::{
    parse_file, Block, Error, Expr, ImplItem, Item, ItemFn, ItemImpl, ItemMod, Stmt, UseTree,
};

const USE_KEYWORDS: &[&str] = &["std", "core", "crate", "self", "alloc", "super"];

pub fn infer_deps(files: &[File]) -> Result<String, syn::Error> {
    let mut deps = vec![];

    files
        .iter()
        .map(|f| -> Result<_, Error> { Ok(parse_file(f.code)?.items) })
        .for_each(|f| {
            if let Ok(tokens) = f {
                // we will keep track of all mod statements used throughout the files
                // if we encounter a dep with the same name as a mod statement
                let mut mod_stmts = vec![];

                tokens.into_iter().for_each(|i| {
                    extract_use(TokenType::Item(i), &mut deps, &mut mod_stmts);
                });

                // remove any deps from deps list if they match a mod stmt
                // this is subject to a limited amount of false positives, but is not too likely to happen in real practice
                deps.retain(|i| !mod_stmts.contains(i));
            }
        });

    // Process `//# ` as a direct statement to put inside depenencies
    // Can only appear at beginning of file
    // stops processing when non ``//# ` is found
    let mut added = 0;
    for file in files {
        for line in file.code.lines() {
            if let Some(line) = line.strip_prefix(r#"//# "#) {
                // find the name of the dependency
                let name = line.find('=').map(|i| line[0..i].trim());

                // remove dependency with same name to avoid conflicts - user provided deps are overrides
                if let Some(name) = name {
                    let index = deps.iter().position(|p| {
                        let convert_case = |b| -> u8 {
                            // only convert - to _ . Else, it's either _, or something we shouldn't filter
                            if b == b'-' {
                                b'_'
                            } else {
                                b
                            }
                        };

                        // Compare crate names with - or _ being equal
                        p.bytes()
                            .map(convert_case)
                            .eq(name.bytes().map(convert_case))
                    });

                    if let Some(i) = index {
                        deps.remove(i);
                    }
                }

                deps.insert(0, line.to_string());
                added += 1;

                continue;
            }

            break;
        }
    }

    for dep in deps.iter_mut().skip(added) {
        dep.push_str(r#" = "*""#)
    }

    Ok(deps.join("\n"))
}

#[derive(Debug)]
enum TokenType {
    // Root item
    Item(Item),
    // Possible token types which can contain a use statement
    Fn(ItemFn),
    Impl(ItemImpl),
    Mod(ItemMod),
    Block(Block),
    Stmt(Stmt),
}

// Once we've found a use statement, extract the ident
fn get_use(tree: UseTree, deps: &mut Vec<String>) {
    match tree {
        UseTree::Path(p) => {
            let ident = p.ident.to_string();

            if !USE_KEYWORDS.contains(&&*ident) && !deps.contains(&ident) {
                deps.push(ident);
            }
        }

        UseTree::Name(n) => {
            let ident = n.ident.to_string();

            if !USE_KEYWORDS.contains(&&*ident) && !deps.contains(&ident) {
                deps.push(ident);
            }
        }

        UseTree::Rename(r) => {
            let ident = r.ident.to_string();

            if !USE_KEYWORDS.contains(&&*ident) && !deps.contains(&ident) {
                deps.push(ident);
            }
        }

        UseTree::Group(g) => {
            for i in g.items {
                match i {
                    UseTree::Path(p) => get_use(UseTree::Path(p), deps),

                    UseTree::Name(n) => get_use(UseTree::Name(n), deps),

                    UseTree::Rename(r) => get_use(UseTree::Rename(r), deps),

                    UseTree::Group(g) => {
                        for tree in g.items {
                            get_use(tree, deps);
                        }
                    }

                    _ => (),
                }
            }
        }

        _ => (),
    }
}

// Go through the entire source code tree to find each use statement, no matter where it is
fn extract_use(item: TokenType, deps: &mut Vec<String>, mod_stmts: &mut Vec<String>) {
    match item {
        TokenType::Item(i) => match i {
            Item::Fn(f) => extract_use(TokenType::Fn(f), deps, mod_stmts),

            Item::Impl(i) => extract_use(TokenType::Impl(i), deps, mod_stmts),

            Item::Mod(m) => {
                mod_stmts.push(m.ident.to_string());

                if m.content.is_some() {
                    extract_use(TokenType::Mod(m), deps, mod_stmts)
                }
            }

            // Finally found a use statement!
            Item::Use(u) => get_use(u.tree, deps),

            _ => (),
        },

        TokenType::Fn(f) => extract_use(TokenType::Block(*f.block), deps, mod_stmts),

        TokenType::Impl(i) => {
            for item in i.items {
                if let ImplItem::Method(method) = item {
                    extract_use(TokenType::Block(method.block), deps, mod_stmts);
                }
            }
        }

        TokenType::Mod(m) => {
            if let Some((_, items)) = m.content {
                for item in items {
                    extract_use(TokenType::Item(item), deps, mod_stmts);
                }
            }
        }

        TokenType::Block(b) => {
            for stmt in b.stmts {
                extract_use(TokenType::Stmt(stmt), deps, mod_stmts);
            }
        }

        TokenType::Stmt(stmt) => match stmt {
            Stmt::Item(i) => extract_use(TokenType::Item(i), deps, mod_stmts),

            Stmt::Expr(e) | Stmt::Semi(e, _) => match e {
                Expr::Async(a) => extract_use(TokenType::Block(a.block), deps, mod_stmts),

                Expr::Block(b) => extract_use(TokenType::Block(b.block), deps, mod_stmts),

                Expr::Closure(c) => {
                    extract_use(TokenType::Stmt(Stmt::Expr(*c.body)), deps, mod_stmts)
                }

                Expr::ForLoop(f) => extract_use(TokenType::Block(f.body), deps, mod_stmts),

                Expr::Group(g) => {
                    extract_use(TokenType::Stmt(Stmt::Expr(*g.expr)), deps, mod_stmts)
                }

                Expr::If(i) => {
                    extract_use(TokenType::Block(i.then_branch), deps, mod_stmts);

                    if i.else_branch.is_some() {
                        extract_use(
                            TokenType::Stmt(Stmt::Expr(*i.else_branch.unwrap().1)),
                            deps,
                            mod_stmts,
                        );
                    }
                }

                Expr::Loop(l) => extract_use(TokenType::Block(l.body), deps, mod_stmts),

                Expr::Match(m) => {
                    for arm in m.arms {
                        extract_use(TokenType::Stmt(Stmt::Expr(*arm.body)), deps, mod_stmts);
                    }
                }

                Expr::TryBlock(t) => extract_use(TokenType::Block(t.block), deps, mod_stmts),

                Expr::Unsafe(u) => extract_use(TokenType::Block(u.block), deps, mod_stmts),

                Expr::While(w) => extract_use(TokenType::Block(w.body), deps, mod_stmts),

                _ => (),
            },

            Stmt::Local(l) => {
                if let Some((_, e)) = l.init {
                    extract_use(TokenType::Stmt(Stmt::Expr(*e)), deps, mod_stmts)
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! try_extract_use {
        ($use_eq:expr, $mod_eq: expr, $code:literal) => {
            let mut deps = vec![];
            let mut mods = vec![];

            let items = parse_file($code).unwrap().items;
            for item in items {
                extract_use(TokenType::Item(item), &mut deps, &mut mods);
            }

            assert_eq!($use_eq as &[&str], &*deps);
            assert_eq!($mod_eq as &[&str], &*mods);
        };
    }

    //
    // Top Level
    //

    #[test]
    fn extract_use_top_level() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use some_lib;
use second_lib;
            "#
        );
    }

    #[test]
    fn extract_use_top_level_with_path() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use some_lib::foobar::Baz;
use second_lib::boobaz;
            "#
        );
    }

    #[test]
    fn extract_use_top_level_with_group() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use some_lib::{
    Bammm
};
use second_lib::boobaz::{
    bamboozle
};
            "#
        );
    }

    #[test]
    fn extract_use_top_level_rename() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use some_lib::bar as baz;
use second_lib::boobaz::bamboozle as haha;
            "#
        );
    }

    #[test]
    fn extract_use_top_fn() {
        try_extract_use!(
            &["nice", "haha"],
            &["bam"],
            r#"
fn foobar() {
    use nice;

    mod bam {
        use haha;
    }
}
            "#
        );
    }

    //
    // Top Level with Use Block
    //

    #[test]
    fn extract_use_top_level_use_block() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use {
    some_lib,
    second_lib
};
            "#
        );
    }

    #[test]
    fn extract_use_top_level_use_block_with_path() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use {
    some_lib::foobar::Baz,
    second_lib::boobaz
};
            "#
        );
    }

    #[test]
    fn extract_use_top_level_use_block_with_group() {
        try_extract_use!(
            &["some_lib", "second_lib"],
            &[],
            r#"
use {
    some_lib::foobar::Baz::{
        Bammm
    },
    second_lib::boobaz::{
        Booze
    }
};
            "#
        );
    }

    //
    // Mod Statement
    //

    #[test]
    fn extract_use_mod_top_level() {
        try_extract_use!(
            &["some_lib", "second_lib", "bar"],
            &["foo", "baz"],
            r#"
mod foo {
    use some_lib;
    use second_lib;
}

mod baz {
    use bar;
}
            "#
        );
    }

    //
    // Impl
    //
    #[test]
    fn extract_use_impl() {
        try_extract_use!(
            &["baz", "impl_boo", "foobam"],
            &[],
            r#"
struct Foo;
impl Foo {
    fn bar() {
        use baz;
        use impl_boo;
    }

    fn bam() {
        use foobam;
    }
}
            "#
        );
    }

    //
    // If / else
    //
    #[test]
    fn extract_use_if_else() {
        try_extract_use!(
            &["haha", "nice"],
            &[],
            r#"
fn foobar() {
    if true {
        use haha;
    } else {
        use nice;
    }
}
            "#
        );
    }

    //
    // Closure
    //
    #[test]
    fn extract_use_closure() {
        try_extract_use!(
            &["closure_test", "closure_2"],
            &[],
            r#"
fn foobar() {
    let b = || {
        use closure_test;
    };

    || {
        use closure_2;
    }
}
            "#
        );
    }

    //
    // Block
    //
    #[test]
    fn extract_use_block() {
        try_extract_use!(
            &["block"],
            &[],
            r#"
fn foobar() {
    {
        use block;
    }
}
            "#
        );
    }

    //
    // Async Block
    //
    #[test]
    fn extract_use_async_block() {
        try_extract_use!(
            &["async_block"],
            &[],
            r#"
fn foobar() {
    async {
        use async_block;
    }
}
            "#
        );
    }

    //
    // For Loop
    //
    #[test]
    fn extract_use_for_loop() {
        try_extract_use!(
            &["a_bus"],
            &[],
            r#"
fn foobar() {
    for i in 0..5 {
        use a_bus;
    }
}
            "#
        );
    }

    //
    // Loop
    //
    #[test]
    fn extract_use_loop() {
        try_extract_use!(
            &["a_car"],
            &[],
            r#"
fn foobar() {
    loop {
        use a_car;
    }
}
            "#
        );
    }

    //
    // Match
    //
    #[test]
    fn extract_use_match() {
        try_extract_use!(
            &["arm", "wrestling"],
            &[],
            r#"
fn foobar() {
    let a = 0;
    match a {
        0 => {
            use arm;
        }

        1 => {
            use wrestling;
        }

        _ => ()
    };
}
            "#
        );
    }

    //
    // While
    //
    #[test]
    fn extract_use_while() {
        try_extract_use!(
            &["it_goes_on_and_on_and_on_my_friends"],
            &[],
            r#"
fn foobar() {
    while True {
        use it_goes_on_and_on_and_on_my_friends;
    }
}
            "#
        );
    }

    //
    // Unsafe
    //
    #[test]
    fn extract_use_unsafe() {
        try_extract_use!(
            &["safety_always"],
            &[],
            r#"
fn foobar() {
    unsafe {
        use safety_always;
    }
}
            "#
        );
    }

    //
    // Try Block, even though these aren't stable
    //
    #[test]
    fn extract_use_try_block() {
        try_extract_use!(
            &["thisisntinstable"],
            &[],
            r#"
fn foobar() {
    try {
        use thisisntinstable;
    }
}
            "#
        );
    }
}
