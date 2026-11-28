//! The Rust sources the architecture rules inspect, parsed with `syn`, plus
//! the small helpers the rules share to read the syntax tree.

use std::path::Path;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ImplItem, Item, Macro, ReturnType, Token, Type, Visibility};

pub struct SourceFile {
    /// Path relative to the repository root, with `/` separators.
    pub path: String,
    pub ast: syn::File,
}

/// An item with its surroundings, from `SourceFile::all_items`.
pub struct Scoped<'a> {
    pub item: &'a Item,
    /// The inline modules around the item, outermost first.
    pub modules: Vec<String>,
    /// Whether the item is test code (in a `#[cfg(test)]` module or a `tests.rs`).
    pub in_test: bool,
}

/// Every `.rs` file under `src/`, read and parsed once per test thread
/// (syn's syntax trees can't be shared between threads).
pub fn repository_sources() -> &'static [SourceFile] {
    thread_local! {
        static SOURCES: &'static [SourceFile] = Box::leak(parse_repository().into_boxed_slice());
    }
    SOURCES.with(|sources| *sources)
}

impl SourceFile {
    pub fn new(path: &str, text: &str) -> Self {
        let ast = syn::parse_file(text).unwrap_or_else(|e| panic!("{path} doesn't parse: {e}"));
        Self {
            path: path.to_string(),
            ast,
        }
    }

    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    pub fn is_under(&self, dir: &str) -> bool {
        self.path.starts_with(dir)
    }

    /// Whether this whole file is test code: a sibling `tests.rs` declared
    /// with `#[cfg(test)] mod tests;`.
    pub fn is_test_file(&self) -> bool {
        self.file_name() == "tests.rs"
    }

    /// Every item of the file, entering inline modules, each with the
    /// modules around it and whether it is test code.
    pub fn all_items(&self) -> Vec<Scoped<'_>> {
        let mut out = Vec::new();
        collect_all(&self.ast.items, &[], self.is_test_file(), &mut out);
        out
    }

    /// The test modules, as (module name, items): the whole file, named
    /// `tests`, for a `tests.rs`; else every inline `#[cfg(test)]` module.
    pub fn test_modules(&self) -> Vec<(String, Vec<&Item>)> {
        if self.is_test_file() {
            return vec![("tests".to_string(), self.ast.items.iter().collect())];
        }
        self.all_items()
            .into_iter()
            .filter_map(|scoped| match scoped.item {
                Item::Mod(module) if is_cfg_test(&module.attrs) => module
                    .content
                    .as_ref()
                    .map(|(_, items)| (module.ident.to_string(), items.iter().collect())),
                _ => None,
            })
            .collect()
    }

    /// The items of the file that are test code.
    pub fn test_items(&self) -> Vec<&Item> {
        self.all_items()
            .into_iter()
            .filter(|scoped| scoped.in_test && !matches!(scoped.item, Item::Mod(_)))
            .map(|scoped| scoped.item)
            .collect()
    }

    pub fn line_of(&self, node: &impl Spanned) -> usize {
        node.span().start().line
    }
}

/// `#[cfg(test)]`.
pub fn is_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && matches!(&attr.meta, syn::Meta::List(list) if list.tokens.to_string() == "test")
    })
}

/// `#[test]` or `#[tokio::test]`.
pub fn is_test_fn(item: &Item) -> bool {
    match item {
        Item::Fn(function) => function.attrs.iter().any(|attr| {
            attr.path().is_ident("test") || path_idents(attr.path()) == ["tokio", "test"]
        }),
        _ => false,
    }
}

pub fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

pub fn is_pub(vis: &Visibility) -> bool {
    !matches!(vis, Visibility::Inherited)
}

/// The identifiers of a path: `crate::x::Foo` → `["crate", "x", "Foo"]`.
pub fn path_idents(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

/// The last identifier of a type's path: `&crate::x::Foo<T>` → `Foo`.
pub fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        Type::Reference(reference) => type_name(&reference.elem),
        _ => String::new(),
    }
}

/// A short rendering of a type for messages: paths, references, tuples and
/// `impl Trait`, generics included.
pub fn render_type(ty: &Type) -> String {
    match ty {
        Type::Path(path) => render_path(&path.path),
        Type::Reference(reference) => format!("&{}", render_type(&reference.elem)),
        Type::Tuple(tuple) => format!(
            "({})",
            tuple
                .elems
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::ImplTrait(bounds) => format!(
            "impl {}",
            bounds
                .bounds
                .iter()
                .filter_map(|bound| match bound {
                    syn::TypeParamBound::Trait(t) => Some(render_path(&t.path)),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" + ")
        ),
        _ => "…".to_string(),
    }
}

pub fn render_return(output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => render_type(ty),
    }
}

/// A short description of an item for messages: `struct Foo`, `impl Bar for Foo`.
pub fn describe(item: &Item) -> String {
    match item {
        Item::Const(c) => format!("const {}", c.ident),
        Item::Static(s) => format!("static {}", s.ident),
        Item::Enum(e) => format!("enum {}", e.ident),
        Item::Struct(s) => format!("struct {}", s.ident),
        Item::Fn(f) => format!("fn {}", f.sig.ident),
        Item::Trait(t) => format!("trait {}", t.ident),
        Item::Type(t) => format!("type {}", t.ident),
        Item::Mod(m) => format!("mod {}", m.ident),
        Item::Impl(i) => match &i.trait_ {
            Some((_, path, _)) => {
                format!("impl {} for {}", render_path(path), type_name(&i.self_ty))
            }
            None => format!("impl {}", type_name(&i.self_ty)),
        },
        Item::Use(_) => "use".to_string(),
        Item::Macro(m) => format!("{}!", path_idents(&m.mac.path).join("::")),
        _ => "item".to_string(),
    }
}

/// The fns of an impl block, as (name, is pub).
pub fn impl_fns(item: &syn::ItemImpl) -> Vec<(String, bool)> {
    item.items
        .iter()
        .filter_map(|impl_item| match impl_item {
            ImplItem::Fn(f) => Some((f.sig.ident.to_string(), is_pub(&f.vis))),
            _ => None,
        })
        .collect()
}

/// A macro's arguments when they are comma-separated expressions
/// (`assert_eq!`, `format!`, `vec!`), so visitors can enter them; empty for
/// any other macro.
pub fn macro_args(mac: &Macro) -> Vec<Expr> {
    mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .map(|args| args.into_iter().collect())
        .unwrap_or_default()
}

fn render_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) => format!(
                "{}<{}>",
                segment.ident,
                args.args
                    .iter()
                    .map(|arg| match arg {
                        syn::GenericArgument::Type(ty) => render_type(ty),
                        _ => "…".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => segment.ident.to_string(),
        })
        .collect::<Vec<_>>()
        .join("::")
}

fn parse_repository() -> Vec<SourceFile> {
    {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut paths = Vec::new();
        collect_rs_files(&root.join("src"), &mut paths);
        paths.sort();
        paths
            .iter()
            .map(|path| {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                SourceFile::new(&relative, &std::fs::read_to_string(path).unwrap())
            })
            .collect()
    }
}

fn collect_all<'a>(
    items: &'a [Item],
    modules: &[String],
    in_test: bool,
    out: &mut Vec<Scoped<'a>>,
) {
    items.iter().for_each(|item| {
        out.push(Scoped {
            item,
            modules: modules.to_vec(),
            in_test,
        });
        if let Item::Mod(module) = item
            && let Some((_, inner)) = &module.content
        {
            let mut path = modules.to_vec();
            path.push(module.ident.to_string());
            collect_all(inner, &path, in_test || is_cfg_test(&module.attrs), out);
        }
    })
}

fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    std::fs::read_dir(dir).unwrap().for_each(|entry| {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_scope_items_inside_test_modules() {
        let file = SourceFile::new(
            "src/a.rs",
            "pub struct S;\n#[cfg(test)]\nmod tests {\n    mod youtube_contract_tests {\n        fn helper() {}\n    }\n}\n",
        );

        assert_eq!(
            file.all_items()
                .iter()
                .map(|s| (describe(s.item), s.modules.join("::"), s.in_test))
                .collect::<Vec<_>>(),
            vec![
                ("struct S".to_string(), String::new(), false),
                ("mod tests".to_string(), String::new(), false),
                (
                    "mod youtube_contract_tests".to_string(),
                    "tests".to_string(),
                    true
                ),
                (
                    "fn helper".to_string(),
                    "tests::youtube_contract_tests".to_string(),
                    true
                ),
            ]
        );
    }

    #[test]
    fn it_should_render_types_for_messages() {
        let file = SourceFile::new(
            "src/a.rs",
            "fn f() -> Result<(StatusCode, Json<T>), ApiError> {}\nfn g() -> impl IntoResponse {}\n",
        );

        assert_eq!(
            file.ast
                .items
                .iter()
                .map(|item| match item {
                    Item::Fn(f) => render_return(&f.sig.output),
                    _ => String::new(),
                })
                .collect::<Vec<_>>(),
            vec![
                "Result<(StatusCode, Json<T>), ApiError>",
                "impl IntoResponse"
            ]
        );
    }
}
