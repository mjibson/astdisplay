#![crate_type = "proc-macro"]
extern crate proc_macro;

use std::{collections::HashMap, fmt::Write};

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Field, Ident, Item, Type};

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

fn is_option(field: &Field) -> bool {
    if let Type::Path(type_path) = &field.ty {
        let segments = &type_path.path.segments;
        return segments[0].ident == "Option";
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
                    writeln!(s, "f.write_node(&self.{});", ident).unwrap();
                }
            }
            writeln!(&mut s, "}} }}").unwrap();

            s.parse().unwrap()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}

struct Attrs(HashMap<String, String>);

impl Attrs {
    fn new(attrs: &[Attribute]) -> Self {
        let mut map = HashMap::new();
        for attr in attrs {
            if !attr.path.is_ident("todoc") {
                continue;
            }
            for tok in attr.tokens.clone().into_iter() {
                // TODO: Surely there's a better way to do this.
                if let quote::__private::TokenTree::Group(group) = tok {
                    let mut toks = group
                        .stream()
                        .into_iter()
                        .map(|tok| tok.to_string())
                        .peekable();
                    while toks.peek().is_some() {
                        let name = toks.next().unwrap();
                        let mut value = "".to_string();
                        if let Some("=") = toks.peek().cloned().as_deref() {
                            toks.next();
                            value = toks.next().unwrap();
                            // Trim off the quotes. Gotta be a better way?
                            value = value[1..value.len() - 1].to_string();
                        }
                        map.insert(name, value);
                        match toks.next().as_deref() {
                            Some(",") => continue,
                            None => break,
                            _ => panic!("unexpected attribute token"),
                        }
                    }
                }
            }
        }
        Self(map)
    }

    fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }
}

impl Drop for Attrs {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            panic!("unknown attributes: {:?}", self.0);
        }
    }
}

const NEST: isize = 4;

/// Returns whether this was a container (Vec, Option).
fn from_field<I: quote::ToTokens>(
    field: &Field,
    ident: &I,
) -> (bool, quote::__private::TokenStream) {
    let mut attrs = Attrs::new(&field.attrs);
    let name = attrs.remove("rename").unwrap_or_else(|| {
        ident
            .to_token_stream()
            .to_string()
            .rsplit(".")
            .next()
            .unwrap()
            .to_uppercase()
            .replace("_", " ")
    });
    if is_bool(&field) {
        (false, quote! { RcDoc::text(#name) })
    } else if is_vec(&field) {
        let sep = match attrs.remove("separator") {
            Some(sep) => quote! { RcDoc::text(#sep) },
            None => quote! { RcDoc::text(",").append(RcDoc::line()) },
        };
        (
            true,
            quote! { if #ident.is_empty() {
                    // TODO: Make sure this doesn't yield an extra space or line
                    // when None. May need to filter out nil docs somewhere.
                    RcDoc::nil()
                } else {
                    let doc = RcDoc::intersperse(
                        #ident.iter().map(|v| v.to_doc()),
                        #sep
                    ).group();
                    RcDoc::text(#name)
                    .append(RcDoc::line())
                    .append(doc)
                    .nest(#NEST)
                    .group()
                }
            },
        )
    } else if is_option(&field) {
        (
            true,
            quote! { if let Some(opt) = &#ident {
                    RcDoc::text(#name)
                    .append(RcDoc::line())
                    .append(opt.to_doc())
                    .nest(#NEST)
                    .group()
                } else {
                    // TODO: Make sure this doesn't yield an extra space or line
                    // when None. May need to filter out nil docs somewhere.
                    RcDoc::nil()
                }
            },
        )
    } else {
        (false, quote! { #ident.to_doc() })
    }
}

#[proc_macro_derive(ToDoc, attributes(todoc))]
pub fn derive_to_doc(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as Item);
    match input {
        Item::Enum(item) => {
            let variants = item.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let mut attrs = Attrs::new(&variant.attrs);
                let name = attrs
                    .remove("rename")
                    .unwrap_or_else(|| ident.to_string().to_uppercase());
                let value = quote! { RcDoc::text(#name) };
                let (fields, value) = match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let fields = fields.named.iter().map(|field| {
                            field.ident.clone().expect("enum variant named field ident")
                        });
                        (quote! { { #(#fields)* } }.into(), value)
                    }
                    syn::Fields::Unnamed(fields) => {
                        let idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(idx, _field)| {
                                // TODO: Better way to do this?
                                Ident::new(&format!("_{idx}"), syn::__private::Span::call_site())
                            })
                            .collect::<Vec<_>>();
                        let value = match fields.unnamed.len() {
                            1 => {
                                let field = fields.unnamed.first().expect("shoulda had one");
                                from_field(field, idents.get(0).unwrap()).1
                            }
                            _ => panic!("exactly 1 unnamed enum variant field supported"),
                        };
                        (quote! { ( #(#idents)* ) }.into(), value)
                    }
                    syn::Fields::Unit => (None, value),
                };
                quote! { Self::#ident #fields => #value, }
            });
            let item_ident = item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            quote! {
                impl #impl_generics ToDoc for #item_ident #ty_generics #where_clause {
                    fn to_doc(&self) -> RcDoc<()> {
                        match self {
                            #(#variants)*
                        }
                    }
                }
            }
            .into()
        }
        Item::Struct(item) => {
            let fields = item.fields.iter().map(|field| {
                let ident = &field.ident;
                let (is_container, mut field) = from_field(field, &quote! { self.#ident });
                if is_container {
                    field = quote! {
                        RcDoc::text(#name)
                        .append(RcDoc::line())
                        .append(#field)
                        .nest(#NEST)
                        .group()
                    };
                }
                quote! { docs.push(#field); }
            });
            let item_ident = item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            let first = fmt_ident(&item_ident);
            quote! {
                impl #impl_generics ToDoc for #item_ident #ty_generics #where_clause {
                    fn to_doc(&self) -> RcDoc<()> {
                        let mut docs = vec![RcDoc::text(#first)];
                        #(#fields)*
                        RcDoc::intersperse(docs, Doc::line()).group()
                    }
                }
            }
            .into()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}
