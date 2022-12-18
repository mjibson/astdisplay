#![crate_type = "proc-macro"]
extern crate proc_macro;

use std::fmt::Write;

use proc_macro::TokenStream;
use syn::{Field, Item, Type};

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

#[proc_macro_derive(AstDisplay)]
pub fn derive_ast_display(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as Item);
    dbg!(&input);
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
            for field in item.fields {
                let ident = field.ident.as_ref().unwrap();
                if is_bool(&field) {
                    writeln!(
                        s,
                        "f.write_str(\"{} \");",
                        ident.to_string().to_uppercase().replace("_", " ")
                    )
                    .unwrap();
                } else if is_vec(&field) {
                    writeln!(
                        s,
                        "f.write_node(&display::comma_separated(&self.{}));",
                        ident
                    )
                    .unwrap();
                } else {
                    panic!("unsupported type for field: {ident}");
                }
            }
            writeln!(&mut s, "}} }}").unwrap();

            s.parse().unwrap()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}
