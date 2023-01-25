use crate::File;

use syn::{parse_file, Block, Error, ImplItem, Item, ItemFn, ItemImpl, ItemMod, Stmt, UseTree};

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

    // Process `//> ` as a direct statement to put inside depenencies
    // Can only appear at beginning of file
    // stops processing when non ``//> ` is found
    let mut added = 0;
    for file in files {
        for line in file.code.lines() {
            if let Some(line) = line.strip_prefix(r#"//> "#) {
                // find the name of the dependency
                let name = line.find('=').map(|i| line[0..i].trim());

                // remove dependency with same name to avoid conflicts - user provided deps are overrides
                if let Some(name) = name {
                    let index = deps.iter().position(|p| p == name);
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
            Item::Fn(_fn) => extract_use(TokenType::Fn(_fn), deps, mod_stmts),

            Item::Impl(_impl) => extract_use(TokenType::Impl(_impl), deps, mod_stmts),

            Item::Mod(_mod) => {
                mod_stmts.push(_mod.ident.to_string());

                if _mod.content.is_some() {
                    extract_use(TokenType::Mod(_mod), deps, mod_stmts)
                }
            }

            // Finally found a use statement!
            Item::Use(u) => get_use(u.tree, deps),

            _ => (),
        },

        TokenType::Fn(_fn) => extract_use(TokenType::Block(*_fn.block), deps, mod_stmts),

        TokenType::Impl(_impl) => {
            for item in _impl.items {
                if let ImplItem::Method(method) = item {
                    extract_use(TokenType::Block(method.block), deps, mod_stmts);
                }
            }
        }

        TokenType::Mod(_mod) => {
            if let Some((_, items)) = _mod.content {
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
                syn::Expr::Async(a) => extract_use(TokenType::Block(a.block), deps, mod_stmts),

                syn::Expr::Block(b) => extract_use(TokenType::Block(b.block), deps, mod_stmts),

                syn::Expr::Closure(c) => {
                    extract_use(TokenType::Stmt(Stmt::Expr(*c.body)), deps, mod_stmts)
                }

                syn::Expr::ForLoop(f) => extract_use(TokenType::Block(f.body), deps, mod_stmts),

                syn::Expr::Group(g) => {
                    extract_use(TokenType::Stmt(Stmt::Expr(*g.expr)), deps, mod_stmts)
                }

                syn::Expr::If(i) => {
                    extract_use(TokenType::Block(i.then_branch), deps, mod_stmts);

                    if i.else_branch.is_some() {
                        extract_use(
                            TokenType::Stmt(Stmt::Expr(*i.else_branch.unwrap().1)),
                            deps,
                            mod_stmts,
                        );
                    }
                }

                syn::Expr::Loop(l) => extract_use(TokenType::Block(l.body), deps, mod_stmts),

                syn::Expr::Match(m) => {
                    for arm in m.arms {
                        extract_use(TokenType::Stmt(Stmt::Expr(*arm.body)), deps, mod_stmts);
                    }
                }

                syn::Expr::TryBlock(t) => extract_use(TokenType::Block(t.block), deps, mod_stmts),

                syn::Expr::Unsafe(u) => extract_use(TokenType::Block(u.block), deps, mod_stmts),

                syn::Expr::While(w) => extract_use(TokenType::Block(w.body), deps, mod_stmts),

                _ => (),
            },

            _ => (),
        },
    }
}
