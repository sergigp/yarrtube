//! The Rust sources the architecture rules inspect, read without a parser
//! dependency: comments and the contents of string and char literals are
//! blanked out (positions and newlines kept), and items are split on the
//! braces and semicolons that are left. `cargo fmt --check` keeps the layout
//! these rules read canonical.

use std::path::Path;
use std::sync::OnceLock;

pub struct SourceFile {
    /// Path relative to the repository root, with `/` separators.
    pub path: String,
    /// The source with comments and literal contents blanked out.
    pub code: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    Use,
    Fn { name: String, is_pub: bool },
    Impl { ty: String, trait_: Option<String> },
    Mod { name: String },
    Trait { name: String },
    Struct { name: String },
    Enum { name: String },
    Other,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    /// Attributes before the item (`#[cfg(test)]`, `#[tokio::test]`, …).
    pub attributes: Vec<String>,
    /// The item's signature: its text up to its body or `;`, attributes left out.
    pub signature: String,
    /// Offset of the item's first character in `SourceFile::code`.
    pub start: usize,
    /// Offsets of the inside of the item's `{ … }` body, if it has one.
    pub body: Option<(usize, usize)>,
}

/// Every `.rs` file under `src/`, read once per test binary.
pub fn repository_sources() -> &'static [SourceFile] {
    static SOURCES: OnceLock<Vec<SourceFile>> = OnceLock::new();
    SOURCES.get_or_init(|| {
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
    })
}

impl SourceFile {
    pub fn new(path: &str, text: &str) -> Self {
        Self {
            path: path.to_string(),
            code: blank_comments_and_literals(text),
        }
    }

    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    pub fn is_under(&self, dir: &str) -> bool {
        self.path.starts_with(dir)
    }

    /// The file's top-level items.
    pub fn items(&self) -> Vec<Item> {
        parse_items(&self.code, 0, self.code.len())
    }

    /// The items inside an item's body (a module, an impl, a trait).
    pub fn children(&self, item: &Item) -> Vec<Item> {
        item.body
            .map(|(start, end)| parse_items(&self.code, start, end))
            .unwrap_or_default()
    }

    pub fn body_of(&self, item: &Item) -> &str {
        item.body
            .map(|(start, end)| &self.code[start..end])
            .unwrap_or("")
    }

    /// Whether this whole file is test code: a sibling `tests.rs` declared
    /// with `#[cfg(test)] mod tests;`.
    pub fn is_test_file(&self) -> bool {
        self.file_name() == "tests.rs"
    }

    /// The test code of the file: the whole file for a `tests.rs`, else the
    /// bodies of its `#[cfg(test)]` modules (nested ones included).
    pub fn test_code(&self) -> Vec<&str> {
        if self.is_test_file() {
            return vec![self.code.as_str()];
        }
        let mut regions = Vec::new();
        self.collect_test_modules(&self.items(), &mut regions);
        regions
    }

    /// Test modules as (module name, items), the whole file being one named
    /// `tests` when it is a `tests.rs`.
    pub fn test_modules(&self) -> Vec<(String, Vec<Item>)> {
        if self.is_test_file() {
            return vec![("tests".to_string(), self.items())];
        }
        let mut modules = Vec::new();
        self.collect_test_module_items(&self.items(), &mut modules);
        modules
    }

    /// Every item of the file, entering modules (not fn bodies), each with
    /// the modules around it and whether it is test code.
    pub fn all_items(&self) -> Vec<Scoped> {
        let mut out = Vec::new();
        self.collect_all(&self.items(), &[], self.is_test_file(), &mut out);
        out
    }

    /// Offsets in `code` where `token` appears as a whole path segment start
    /// (not preceded by an identifier character or `::`).
    pub fn token_offsets(&self, token: &str) -> Vec<usize> {
        token_offsets(&self.code, token)
    }

    pub fn line_of(&self, offset: usize) -> usize {
        self.code[..offset].matches('\n').count() + 1
    }

    fn collect_all(
        &self,
        items: &[Item],
        modules: &[String],
        in_test: bool,
        out: &mut Vec<Scoped>,
    ) {
        items.iter().for_each(|item| {
            out.push(Scoped {
                item: item.clone(),
                modules: modules.to_vec(),
                in_test,
            });
            if let ItemKind::Mod { name } = &item.kind {
                let mut inner = modules.to_vec();
                inner.push(name.clone());
                self.collect_all(
                    &self.children(item),
                    &inner,
                    in_test || item.is_cfg_test(),
                    out,
                );
            }
        })
    }

    fn collect_test_modules<'a>(&'a self, items: &[Item], regions: &mut Vec<&'a str>) {
        items.iter().for_each(|item| match &item.kind {
            ItemKind::Mod { .. } if item.is_cfg_test() => regions.push(self.body_of(item)),
            ItemKind::Mod { .. } => self.collect_test_modules(&self.children(item), regions),
            _ => {}
        })
    }

    fn collect_test_module_items(&self, items: &[Item], modules: &mut Vec<(String, Vec<Item>)>) {
        items.iter().for_each(|item| {
            if let ItemKind::Mod { name } = &item.kind {
                if item.is_cfg_test() && item.body.is_some() {
                    modules.push((name.clone(), self.children(item)));
                }
                self.collect_test_module_items(&self.children(item), modules);
            }
        })
    }
}

/// An item with its surroundings, from `SourceFile::all_items`.
#[derive(Clone, Debug)]
pub struct Scoped {
    pub item: Item,
    /// The inline modules around the item, outermost first.
    pub modules: Vec<String>,
    /// Whether the item is test code (`#[cfg(test)]` module or `tests.rs`).
    pub in_test: bool,
}

impl Item {
    /// The return type of a fn signature, squashed (`""` when none).
    pub fn return_type(&self) -> String {
        let signature = self.signature.split(" where ").next().unwrap_or("");
        let mut depth = 0i32;
        let bytes = signature.as_bytes();
        let arrow = (0..bytes.len()).find(|&i| {
            match bytes[i] {
                b'(' | b'<' if i == 0 || bytes[i - 1] != b'-' => depth += 1,
                b')' => depth -= 1,
                b'>' if bytes[i - 1] != b'-' => depth -= 1,
                _ => {}
            }
            depth == 0 && signature[i..].starts_with("->")
        });
        arrow
            .map(|i| signature[i + 2..].trim().to_string())
            .unwrap_or_default()
    }

    pub fn has_attribute(&self, prefix: &str) -> bool {
        self.attributes.iter().any(|a| a.starts_with(prefix))
    }

    pub fn is_cfg_test(&self) -> bool {
        self.has_attribute("#[cfg(test)]")
    }

    pub fn is_test_fn(&self) -> bool {
        matches!(self.kind, ItemKind::Fn { .. })
            && (self.has_attribute("#[test]") || self.has_attribute("#[tokio::test"))
    }
}

/// Collapses runs of whitespace into one space.
pub fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// Replaces comments and the contents of string/char literals with spaces,
/// keeping newlines, quotes and every byte offset.
fn blank_comments_and_literals(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                push_blank(&mut out, chars[i]);
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            loop {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    push_blank(&mut out, chars[i]);
                    i += 1;
                }
            }
        } else if let Some(hashes) = raw_string_start(&chars, i) {
            // r"…", r#"…"#, br#"…"#: keep the prefix and quotes.
            while chars[i] != '"' {
                out.push(chars[i]);
                i += 1;
            }
            out.push('"');
            i += 1;
            let closing: String = std::iter::once('"')
                .chain((0..hashes).map(|_| '#'))
                .collect();
            let closing: Vec<char> = closing.chars().collect();
            while !chars[i..].starts_with(&closing) {
                push_blank(&mut out, chars[i]);
                i += 1;
            }
            closing.iter().for_each(|c| out.push(*c));
            i += closing.len();
        } else if c == '"' {
            out.push('"');
            i += 1;
            while chars[i] != '"' {
                if chars[i] == '\\' {
                    push_blank(&mut out, chars[i]);
                    i += 1;
                }
                push_blank(&mut out, chars[i]);
                i += 1;
            }
            out.push('"');
            i += 1;
        } else if c == '\'' && is_char_literal(&chars, i) {
            out.push('\'');
            i += 1;
            while chars[i] != '\'' {
                if chars[i] == '\\' {
                    push_blank(&mut out, chars[i]);
                    i += 1;
                }
                push_blank(&mut out, chars[i]);
                i += 1;
            }
            out.push('\'');
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn push_blank(out: &mut String, c: char) {
    if c == '\n' {
        out.push('\n');
    } else {
        (0..c.len_utf8()).for_each(|_| out.push(' '));
    }
}

fn prev_char(chars: &[char], i: usize) -> Option<char> {
    i.checked_sub(1).map(|p| chars[p])
}

fn is_ident_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// `r"`, `r#"`, `br"`, `br##"`… not preceded by an identifier character.
fn raw_string_start(chars: &[char], i: usize) -> Option<usize> {
    let start = if chars[i] == 'b' && chars.get(i + 1) == Some(&'r') {
        i + 1
    } else if chars[i] == 'r' {
        i
    } else {
        return None;
    };
    if is_ident_char(prev_char(chars, i)) {
        return None;
    }
    let hashes = chars[start + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(start + 1 + hashes) == Some(&'"')).then_some(hashes)
}

/// A char literal (`'a'`, `'\n'`, `'\u{1F600}'`) rather than a lifetime (`'a`).
fn is_char_literal(chars: &[char], i: usize) -> bool {
    match chars.get(i + 1) {
        Some('\\') => true,
        Some(_) => chars.get(i + 2) == Some(&'\''),
        None => false,
    }
}

/// Splits `code[start..end]` into items at depth 0: on `;` and on `{ … }`
/// bodies (a body followed by `;`, as in `const X: T = T { … };`, stays one
/// item). Parentheses and brackets are skipped whole.
fn parse_items(code: &str, start: usize, end: usize) -> Vec<Item> {
    let bytes = code.as_bytes();
    let mut items = Vec::new();
    let mut item_start = start;
    let mut i = start;
    while i < end {
        match bytes[i] {
            b';' => {
                push_item(code, item_start, i, None, &mut items);
                item_start = i + 1;
            }
            b'{' => {
                let close = matching_close(bytes, i);
                let mut after = close + 1;
                while after < end && (bytes[after] as char).is_whitespace() {
                    after += 1;
                }
                let header_end = i;
                push_item(
                    code,
                    item_start,
                    header_end,
                    Some((i + 1, close)),
                    &mut items,
                );
                if after < end && bytes[after] == b';' {
                    after += 1;
                } else {
                    after = close + 1;
                }
                i = after;
                item_start = after;
                continue;
            }
            b'(' | b'[' => {
                i = matching_close(bytes, i);
            }
            _ => {}
        }
        i += 1;
    }
    items
}

fn matching_close(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    loop {
        match bytes[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn push_item(
    code: &str,
    start: usize,
    header_end: usize,
    body: Option<(usize, usize)>,
    items: &mut Vec<Item>,
) {
    let header = &code[start..header_end];
    let leading = header.len() - header.trim_start().len();
    let (attributes, signature) = split_attributes(header.trim());
    if signature.is_empty() && attributes.is_empty() && body.is_none() {
        return;
    }
    items.push(Item {
        kind: item_kind(&signature),
        attributes,
        signature,
        start: start + leading,
        body,
    });
}

fn split_attributes(header: &str) -> (Vec<String>, String) {
    let mut attributes = Vec::new();
    let mut rest = header.trim_start();
    while rest.starts_with("#[") || rest.starts_with("#![") {
        let open = rest.find('[').unwrap();
        let close = matching_close(rest.as_bytes(), open);
        attributes.push(squash(&rest[..=close]));
        rest = rest[close + 1..].trim_start();
    }
    (attributes, squash(rest))
}

fn item_kind(signature: &str) -> ItemKind {
    let (is_pub, rest) = strip_visibility(signature);
    let rest = strip_keywords(
        rest,
        &[
            "default ",
            "const ",
            "async ",
            "unsafe ",
            "extern \"\" ",
            "extern \"C\" ",
        ],
    );
    if let Some(name) = keyword_ident(rest, "fn ") {
        return ItemKind::Fn { name, is_pub };
    }
    if rest.starts_with("use ") {
        return ItemKind::Use;
    }
    if let Some(name) = keyword_ident(rest, "mod ") {
        return ItemKind::Mod { name };
    }
    if let Some(name) = keyword_ident(rest, "trait ") {
        return ItemKind::Trait { name };
    }
    if let Some(name) = keyword_ident(rest, "struct ") {
        return ItemKind::Struct { name };
    }
    if let Some(name) = keyword_ident(rest, "enum ") {
        return ItemKind::Enum { name };
    }
    if rest.starts_with("impl") && !is_ident_char(rest.chars().nth(4)) {
        return impl_kind(&rest[4..]);
    }
    ItemKind::Other
}

fn strip_visibility(signature: &str) -> (bool, &str) {
    match signature.strip_prefix("pub") {
        Some(rest) if rest.starts_with('(') => {
            let close = rest.find(')').unwrap();
            (true, rest[close + 1..].trim_start())
        }
        Some(rest) if rest.starts_with(' ') => (true, rest.trim_start()),
        _ => (false, signature),
    }
}

fn strip_keywords<'a>(mut rest: &'a str, keywords: &[&str]) -> &'a str {
    while let Some(stripped) = keywords.iter().find_map(|k| rest.strip_prefix(k)) {
        rest = stripped;
    }
    rest
}

fn keyword_ident(rest: &str, keyword: &str) -> Option<String> {
    rest.strip_prefix(keyword).map(|after| {
        after
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    })
}

/// `impl<T> Trait<T> for Type<T> where …` → (Type, Some(Trait)).
fn impl_kind(after_impl: &str) -> ItemKind {
    let rest = skip_generics(after_impl.trim_start());
    let rest = rest.split(" where ").next().unwrap_or(rest);
    match split_top_level_for(rest) {
        Some((trait_, ty)) => ItemKind::Impl {
            ty: base_name(ty),
            trait_: Some(base_name(trait_)),
        },
        None => ItemKind::Impl {
            ty: base_name(rest),
            trait_: None,
        },
    }
}

fn skip_generics(text: &str) -> &str {
    if !text.starts_with('<') {
        return text;
    }
    let mut depth = 0;
    for (i, c) in text.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return text[i + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    text
}

fn split_top_level_for(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let bytes = text.as_bytes();
    (0..bytes.len()).find_map(|i| {
        match bytes[i] {
            b'<' | b'(' => depth += 1,
            b'>' | b')' => depth -= 1,
            _ => {}
        }
        (depth == 0 && text[i..].starts_with(" for ")).then(|| (&text[..i], &text[i + 5..]))
    })
}

/// The last path segment of a type, without generics or references:
/// `&'a crate::x::Foo<T>` → `Foo`.
fn base_name(ty: &str) -> String {
    let mut ty = ty.trim().trim_start_matches('&').trim_start();
    if ty.starts_with('\'') {
        ty = ty.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
    }
    let path: String = ty
        .trim_start_matches("mut ")
        .trim_start_matches("dyn ")
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect();
    path.rsplit("::").next().unwrap_or("").to_string()
}

fn token_offsets(code: &str, token: &str) -> Vec<usize> {
    code.match_indices(token)
        .filter(|(i, _)| {
            let before = code[..*i].chars().next_back();
            !is_ident_char(before) && !code[..*i].ends_with("::")
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_blank_comments_and_literal_contents_keeping_offsets() {
        let text = "fn f<'a>(x: &'a str) -> char { // a { comment\n    let s = r#\"{ \"raw\" }\"#; /* { */ '{' }";

        let file = SourceFile::new("src/a.rs", text);

        assert_eq!(
            file.code,
            "fn f<'a>(x: &'a str) -> char {               \n    let s = r#\"         \"#;         ' ' }"
        );
    }

    #[test]
    fn it_should_split_items_and_read_their_kinds() {
        let file = SourceFile::new(
            "src/a.rs",
            "use std::fmt;\nconst A: Foo = Foo { a: [1; 3] };\n#[derive(Debug)]\npub(crate) struct S;\n\
             impl<T: Clone> fmt::Display for Wrapper<T> where T: Copy {\n    fn fmt(&self) {}\n}\n\
             #[cfg(test)]\nmod tests;\n",
        );

        assert_eq!(
            file.items()
                .into_iter()
                .map(|item| (item.kind, item.attributes))
                .collect::<Vec<_>>(),
            vec![
                (ItemKind::Use, vec![]),
                (ItemKind::Other, vec![]),
                (
                    ItemKind::Struct { name: "S".into() },
                    vec!["#[derive(Debug)]".to_string()]
                ),
                (
                    ItemKind::Impl {
                        ty: "Wrapper".into(),
                        trait_: Some("Display".into())
                    },
                    vec![]
                ),
                (
                    ItemKind::Mod {
                        name: "tests".into()
                    },
                    vec!["#[cfg(test)]".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn it_should_read_a_fn_return_type() {
        let file = SourceFile::new(
            "src/a.rs",
            "pub async fn f<F: Fn(u8) -> u8>(f: F) -> Result<(StatusCode, Json<T>), ApiError> where F: Copy {}",
        );

        assert_eq!(
            file.items()
                .into_iter()
                .map(|item| item.return_type())
                .collect::<Vec<_>>(),
            vec!["Result<(StatusCode, Json<T>), ApiError>"]
        );
    }
}
