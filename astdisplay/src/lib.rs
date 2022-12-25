#![crate_type = "proc-macro"]
extern crate proc_macro;

use std::{collections::HashMap, fmt::Write};

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Item, Type};

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

    fn name(&mut self, mut doc: TokenStream2, name: &str) -> TokenStream2 {
        let name = self.remove("rename").unwrap_or_else(|| name.to_string());
        if self.remove("no_name").is_none() {
            doc = quote! { #doc.map(|doc|
                pretty::RcDoc::text(#name)
                .append(pretty::RcDoc::line())
                .append(doc)
                .nest(#NEST)
                //.group()
            ) };
        }
        doc
    }

    fn prefix(&mut self, mut doc: TokenStream2) -> TokenStream2 {
        if let Some(prefix) = self.remove("prefix") {
            doc = quote! { #doc.map(|doc| pretty::RcDoc::text(#prefix).append(doc)) };
        }
        doc
    }

    fn suffix(&mut self, mut doc: TokenStream2) -> TokenStream2 {
        if let Some(suffix) = self.remove("suffix") {
            doc = quote! { #doc.map(|doc| doc.append(pretty::RcDoc::text(#suffix))) };
        }
        doc
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

#[proc_macro_derive(ToDoc, attributes(todoc))]
pub fn derive_to_doc(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as Item);
    match input {
        Item::Enum(item) => {
            let _enum_attrs = Attrs::new(&item.attrs);
            let variants = item.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let mut variant_attrs = Attrs::new(&variant.attrs);
                let name = variant_attrs
                    .remove("rename")
                    .unwrap_or_else(|| fmt_ident(&variant.ident));
                let FromFields { fields, doc } = from_fields(&variant.fields, &name);
                quote! { Self::#ident #fields => #doc.unwrap_or_else(pretty::RcDoc::nil), }
            });
            let item_ident = item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            quote! {
                impl #impl_generics ToDoc for #item_ident #ty_generics #where_clause {
                    fn to_doc(&self) -> pretty::RcDoc<()> {
                        match self {
                            #(#variants)*
                        }
                    }
                }
            }
            .into()
        }
        Item::Struct(item) => {
            let mut struct_attrs = Attrs::new(&item.attrs);
            let name = fmt_ident(&item.ident);
            let FromFields { fields, doc } = from_fields(&item.fields, &name);
            let item_ident = item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            let doc = struct_attrs.name(doc, &name);
            quote! {
                impl #impl_generics ToDoc for #item_ident #ty_generics #where_clause {
                    fn to_doc(&self) -> pretty::RcDoc<()> {
                        let Self #fields = self;
                        #doc.unwrap_or_else(pretty::RcDoc::nil)
                    }
                }
            }
            .into()
        }
        _ => panic!("unsupported: {:?}", input),
    }
}

type TokenStream2 = proc_macro2::TokenStream;

struct FromField {
    doc: TokenStream2,
    _attrs: Attrs,
}

// ident is something like self.blah, name is blah.
fn from_field(field: &Field, ident: &Ident, name: &str) -> FromField {
    let mut attrs = Attrs::new(&field.attrs);
    let doc = if is_bool(&field) {
        quote! { #ident.then(|| pretty::RcDoc::text(#name)) }
    } else if is_vec(&field) {
        let sep = match attrs.remove("separator") {
            Some(sep) => quote! { pretty::RcDoc::text(#sep) },
            None => quote! { pretty::RcDoc::text(",").append(pretty::RcDoc::line()) },
        };
        let doc = quote! { if #ident.is_empty() {
                None
            } else {
                Some(
                    pretty::RcDoc::intersperse(
                        #ident.iter().map(|v| v.to_doc()),
                        #sep
                    )
                    .group()
                )
            }
        };
        let doc = attrs.name(doc, &name);
        doc
    } else if is_option(&field) {
        let doc = quote! { #ident.as_ref().map(|opt| opt.to_doc()) };
        let doc = attrs.name(doc, &name);
        doc
    } else {
        quote! { Some(#ident.to_doc()) }
    };
    let doc = attrs.prefix(doc);
    let doc = attrs.suffix(doc);
    FromField { doc, _attrs: attrs }
}

struct FromFields {
    fields: TokenStream2,
    doc: TokenStream2,
}

fn from_fields(fields: &Fields, name: &str) -> FromFields {
    match fields {
        Fields::Named(fields) => named_fields(fields),
        Fields::Unnamed(fields) => unnamed_fields(fields, name),
        Fields::Unit => FromFields {
            fields: quote! {},
            doc: quote! { Some(pretty::RcDoc::text(#name)) },
        },
    }
}

fn unnamed_fields(fields: &FieldsUnnamed, name: &str) -> FromFields {
    let idents = (0..fields.unnamed.len())
        .map(|i| {
            // TODO: Better way to do this?
            Ident::new(&format!("_{i}"), syn::__private::Span::call_site())
        })
        .collect::<Vec<_>>();
    let doc = match fields.unnamed.len() {
        0 => quote! { Some(pretty::RcDoc::text(#name)) },
        1 => {
            let field = from_field(fields.unnamed.first().unwrap(), &idents[0], name);
            field.doc
        }
        _ => panic!(
            "unsupported: unnamed fields with len {}",
            fields.unnamed.len()
        ),
    };
    let idents = quote! { (#(#idents),*) };
    FromFields {
        fields: idents,
        doc,
    }
}

fn named_fields(fields: &FieldsNamed) -> FromFields {
    let docs = fields.named.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let FromField { doc, _attrs } = from_field(field, ident, &fmt_ident(ident));
        doc
    });
    let doc = quote! { {
       let docs = [#(#docs),*].into_iter().filter_map(|v| v).collect::<Vec<_>>();
       if docs.is_empty() {
           None
       } else {
           Some(pretty::RcDoc::intersperse(docs, pretty::RcDoc::line()).group())
       }
    } };
    let idents = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().unwrap());
    let idents = quote! { {#(#idents),*} };
    FromFields {
        fields: idents,
        doc,
    }
}

/*

For a struct with {..} fields: apply the rules for {..}
For a struct with () fields:
For a enum: current variant converted to doc

Converting a Field to a doc:

() [empty variant or tuple or struct()]: enum variant name or struct field name
bool: field name if true, nil if false
struct/enum: recursive call
{..}: for each field in order, convert to doc, then intersperse with line
Option<T>: nil if None, otherwise field name nested with T converted to doc
Vec<T>: nil if empty, otherwise field name nested with values converted to docs, interspersed with comma line.

*/
