#![crate_type = "proc-macro"]
extern crate proc_macro;

use std::{collections::HashMap, fmt::Write};

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Field, Fields, Ident, Item, Type};

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

struct FromField {
    is_container: bool,
    name: String,
    doc: quote::__private::TokenStream,
    attrs: Attrs,
}

/// Returns whether this was a container (Vec, Option).
fn from_field<I: quote::ToTokens>(field: &Field, ident: &I) -> FromField {
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
    let (is_container, doc) = if is_bool(&field) {
        (false, quote! { Some(RcDoc::text(#name)) })
    } else if is_vec(&field) {
        let sep = match attrs.remove("separator") {
            Some(sep) => quote! { RcDoc::text(#sep) },
            None => quote! { RcDoc::text(",").append(RcDoc::line()) },
        };
        (
            true,
            quote! { if #ident.is_empty() {
                    None
                } else {
                    Some(RcDoc::intersperse(
                        #ident.iter().map(|v| v.to_doc()),
                        #sep
                    ).group())
                }
            },
        )
    } else if is_option(&field) {
        let mut doc = quote! { if let Some(opt) = &#ident {
                Some(opt.to_doc())
            } else {
                None
            }
        };
        // TODO: This should be a rename. SelectItem::Expr.alias should by
        // default spit out "ALIAS" because it's an Option.
        if let Some(prefix) = attrs.remove("prefix") {
            doc = quote! { #doc.map(|doc|
                RcDoc::text(#prefix)
                .append(RcDoc::line())
                .append(doc)
                .nest(#NEST)
                .group()
            ) };
        }
        (true, doc)
    } else {
        (false, quote! { Some(#ident.to_doc()) })
    };
    FromField {
        is_container,
        name,
        doc,
        attrs,
    }
}

fn extract_fields(
    fields: &Fields,
    name: &str,
) -> (Option<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    match fields {
        syn::Fields::Named(fields) => {
            let idents = fields
                .named
                .iter()
                .map(|field| &field.ident)
                .collect::<Vec<_>>();
            let fields = fields.named.iter().map(|field| {
                let doc = from_field(field, &field.ident).doc;
                quote! { #doc.unwrap_or_else(RcDoc::nil) }
            });
            let value = quote! { {
                RcDoc::intersperse([#(#fields),*], RcDoc::line())
                .nest(#NEST)
                .group()
            } }
            .into();
            (Some(quote! { { #(#idents),* } }.into()), value)
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
                    let doc = from_field(field, idents.get(0).unwrap()).doc;
                    quote! { #doc.unwrap_or_else(RcDoc::nil) }
                }
                _ => panic!("exactly 1 unnamed enum variant field supported"),
            };
            (Some(quote! { ( #(#idents)* ) }.into()), value.into())
        }
        syn::Fields::Unit => (None, quote! { RcDoc::text(#name) }.into()),
    }
}

#[proc_macro_derive(ToDoc, attributes(todoc))]
pub fn derive_to_doc(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as Item);
    match input {
        Item::Enum(item) => {
            let mut enum_attrs = Attrs::new(&item.attrs);
            let variants = item.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let mut attrs = Attrs::new(&variant.attrs);
                let name = attrs
                    .remove("rename")
                    .unwrap_or_else(|| ident.to_string().to_uppercase());
                let (fields, value) = extract_fields(&variant.fields, &name);
                let v = quote! { Self::#ident #fields => #value, };
                println!("VARIANT: {v}");
                v
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
            dbg!(&item);
            let mut struct_attrs = Attrs::new(&item.attrs);
            let fields = item.fields.iter().map(|field| {
                let ident = &field.ident;
                let FromField {
                    is_container,
                    name,
                    mut doc,
                    attrs: _,
                } = from_field(field, &quote! { self.#ident });
                if is_container {
                    doc = quote! {
                        #doc.map(|doc|
                            RcDoc::text(#name)
                            .append(RcDoc::line())
                            .append(doc)
                            .nest(#NEST)
                            .group()
                        )
                    };
                }
                quote! { docs.push(#doc); }
            });
            let item_ident = item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            let first = if struct_attrs.remove("unnamed").is_some() {
                quote! { None }
            } else {
                let first = fmt_ident(&item_ident);
                quote! { Some(RcDoc::text(#first)) }
            };
            quote! {
                impl #impl_generics ToDoc for #item_ident #ty_generics #where_clause {
                    fn to_doc(&self) -> RcDoc<()> {
                        let mut docs = vec![#first];
                        #(#fields)*
                        let docs = docs.into_iter().filter_map(|i| i);
                        RcDoc::intersperse(docs, Doc::line()).group()
                    }
                }
            }
            .into()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}
