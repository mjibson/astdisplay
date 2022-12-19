#![crate_type = "proc-macro"]
extern crate proc_macro;

use std::fmt::Write;

use proc_macro::TokenStream;
use syn::{Field, Ident, Item, Type};

fn is_bool(field: &Field) -> bool {
    matches!(
        &field.ty,
        Type::Path(type_path) if type_path.path.is_ident("bool"),
    )
}

fn is_vec(field: &Field) -> bool {
    if let Type::Path(type_path) = &field.ty {
        let segments = &type_path.path.segments;
        return segments[0].ident == "Vec";
    }
    false
}

fn truncate_stmt_suffix(s: &str) -> &str {
    s.trim_end_matches("sStatement")
        .trim_end_matches("Statement")
}

fn split_upper<'a>(mut s: &'a str) -> Vec<&'a str> {
    let mut indexes = Vec::new();
    for (i, c) in s.chars().enumerate().skip(1) {
        if c.is_uppercase() {
            indexes.push(i);
        }
    }
    let mut strs = Vec::with_capacity(indexes.len());
    for split_at in indexes {
        let (l, r) = s.split_at(split_at);
        s = r;
        strs.push(l);
    }
    strs.push(s);
    dbg!(&strs);
    strs
}

fn fmt_ident(ident: &Ident) -> String {
    split_upper(truncate_stmt_suffix(&ident.to_string()))
        .join(" ")
        .to_uppercase()
}

#[proc_macro_derive(AstDisplay)]
pub fn derive_ast_display(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as Item);
    match input {
        Item::Enum(item) => {
            let mut s = format!(
                "
impl AstDisplay for {} {{
    fn fmt<W>(&self, f: &mut AstFormatter<W>)
    where
        W: fmt::Write {{
        match self {{",
                item.ident
            );
            for variant in item.variants {
                writeln!(
                    &mut s,
                    "Self::{} => f.write_str(\"{}\"),",
                    variant.ident,
                    variant.ident.to_string().to_uppercase()
                )
                .unwrap();
            }
            writeln!(&mut s, "}} }} }}").unwrap();

            s.parse().unwrap()
        }
        Item::Struct(item) => {
            let mut s = format!(
                "
impl AstDisplay for {} {{
    fn fmt<W>(&self, f: &mut AstFormatter<W>)
    where
        W: fmt::Write {{\n",
                item.ident
            );
            writeln!(s, "f.write_str(\"{} \");", fmt_ident(&item.ident)).unwrap();
            for (idx, field) in item.fields.iter().enumerate() {
                if idx > 0 {
                    writeln!(s, "f.write_str(\" \");").unwrap();
                }
                let ident = field.ident.as_ref().unwrap();
                if is_bool(&field) {
                    writeln!(
                        s,
                        "f.write_str(\"{}\");",
                        ident.to_string().to_uppercase().replace("_", " ")
                    )
                    .unwrap();
                } else if is_vec(&field) {
                    writeln!(
                        s,
                        "f.write_node(&mz_sql_parser::ast::display::comma_separated(&self.{}));",
                        ident
                    )
                    .unwrap();
                } else {
                    //panic!("unsupported type for field: {ident}");
                    writeln!(s, "f.write_node(&self.{});", ident).unwrap();
                }
            }
            writeln!(&mut s, "}} }}").unwrap();

            s.parse().unwrap()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}
