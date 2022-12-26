#![allow(dead_code, unused_imports, unused_variables)]

use std::fmt;

use mz_sql_parser::ast::display::AstDisplay;
use mz_sql_parser::ast::{display::AstFormatter, Ident};
use mz_sql_parser::ast::{AstInfo, UnresolvedDatabaseName, UnresolvedObjectName};
use pretty::{Doc, RcDoc};

use astdisplay::*;

trait ToDoc {
    fn to_doc(&self) -> RcDoc<()>;
}

#[derive(AstDisplay, ToDoc)]
enum Blah {
    Yo,
    Foo,
}

impl ToDoc for Ident {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.as_str())
    }
}

impl ToDoc for UnresolvedObjectName {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.to_ast_string())
    }
}

impl ToDoc for UnresolvedDatabaseName {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.to_ast_string())
    }
}

impl ToDoc for usize {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.to_string())
    }
}

/*
#[derive(AstDisplay, ToDoc)]
pub struct DropRolesStatement {
    pub if_exists: bool,
    pub names: Vec<Ident>,
}

#[derive(AstDisplay, ToDoc)]
struct AlterConnectionStatement {
    pub if_exists: bool,
    pub name: UnresolvedObjectName,
}

#[derive(AstDisplay, ToDoc)]
pub struct DiscardStatement {
    pub target: DiscardTarget,
}

#[derive(AstDisplay, ToDoc)]
pub enum DiscardTarget {
    Plans,
    Sequences,
    Temp,
    All,
}

#[derive(AstDisplay, ToDoc)]
pub struct DropDatabaseStatement {
    pub if_exists: bool,
    pub name: UnresolvedDatabaseName,
    pub restrict: bool,
}
*/

fn main() {
    let expr = Expr::Identifier(vec!["blah".into(), "second".into()]);
    /*
    let s = DropRolesStatement {
        if_exists: true,
        names: vec![Ident::from("one"), Ident::from("two")],
    };
    let s = AlterConnectionStatement {
        name: UnresolvedObjectName::unqualified("naaame"),
        if_exists: true,
    };
    let s = DiscardStatement {
        target: DiscardTarget::Sequences,
    };
    let s = DropDatabaseStatement {
        name: UnresolvedDatabaseName(Ident::from("db")),
        if_exists: true,
        restrict: true,
    };
    */
    let s = Values(vec![
        Value(vec![expr.clone(), expr.clone()]),
        Value(vec![
            Expr::Unit,
            Expr::Struct { a: true, b: None },
            Expr::Struct {
                a: false,
                b: Some("bb".into()),
            },
        ]),
    ]);
    let s = Select {
        projection: vec![
            SelectItem::Wildcard,
            SelectItem::Expr {
                expr: expr.clone(),
                alias: None,
            },
        ],
        selection: Some(expr.clone()),
        //group_by: vec![expr.clone()],
        group_by: Vec::new(),
        having: Some(expr.clone()),
    };
    let s = Expr::List(vec![]);

    // let ast = s.to_ast_string();
    // println!("{}", ast);
    let mut prev = "".to_string();
    let doc = s.to_doc();
    for i in 1..=100 {
        let mut cur = Vec::new();
        doc.render(i, &mut cur).unwrap();
        let cur = String::from_utf8(cur).unwrap();
        if cur != prev {
            prev = cur;
            println!("\n{i}:\n{prev}");
        }
    }
}

#[derive(ToDoc)]
#[todoc(no_name)]
struct Select /*<T: AstInfo>*/ {
    //pub distinct: Option<Distinct<T>>,
    #[todoc(rename = "SELECT")]
    pub projection: Vec<SelectItem>,
    //pub from: Vec<TableWithJoins<T>>,
    #[todoc(rename = "WHERE")]
    pub selection: Option<Expr>,
    pub group_by: Vec<Expr>,
    #[todoc(ignore)]
    pub having: Option<Expr>,
    //pub options: Vec<SelectOption<T>>,
}

#[derive(ToDoc)]
//#[todoc(no_name)]
struct Value(#[todoc(prefix = "(", suffix = ")", no_name)] Vec<Expr>);

#[derive(ToDoc)]
struct Values(Vec<Value>);

#[derive(ToDoc)]
enum SelectItem /*<T: AstInfo>*/ {
    /// An expression, optionally followed by `[ AS ] alias`.
    Expr { expr: Expr, alias: Option<Ident> },
    /// An unqualified `*`.
    #[todoc(rename = "*")]
    Wildcard,
}

#[derive(Clone, ToDoc)]
#[todoc(no_name)]
struct CaseCondition {
    #[todoc(nest = "WHEN")]
    when: Expr,
    #[todoc(nest = "THEN")]
    then: Expr,
}

#[derive(Clone, ToDoc)]
enum Expr {
    List(#[todoc(prefix = "LIST[", suffix = "]", no_name, show_empty)] Vec<Expr>),
    /// `CASE [<operand>] WHEN <condition> THEN <result> ... [ELSE <result>] END`
    #[todoc(nest = "CASE", nest_suffix = "END")]
    Case {
        #[todoc(no_name)]
        operand: Option<Box<Expr>>,
        #[todoc(separator = "", no_name)]
        conditions: Vec<CaseCondition>,
        #[todoc(nest = "ELSE", no_name)]
        else_result: Option<Box<Expr>>,
    },
    /// `<expr> [ NOT ] {LIKE, ILIKE} <pattern> [ ESCAPE <escape> ]`
    Like {
        expr: Box<Expr>,
        #[todoc(rename = "NOT")]
        negated: bool,
        #[todoc(rename = "ILIKE", else = "LIKE")]
        case_insensitive: bool,
        pattern: Box<Expr>,
        escape: Option<Box<Expr>>,
    },
    Unit,
    Struct {
        a: bool,
        b: Option<Ident>,
    },
    /// Identifier e.g. table name or column name
    Identifier(#[todoc(separator = ".", no_name, separator_noline)] Vec<Ident>),
    ExpectedGroupSizeYo,
    #[todoc(prefix = "$")]
    Parameter(usize),
    Not {
        #[todoc(nest = "NOT")]
        expr: Box<Expr>,
    },
    And {
        left: Box<Expr>,
        #[todoc(nest = "AND")]
        right: Box<Expr>,
    },
}
