use std::fmt;

use mz_sql_parser::ast::display::AstDisplay;
use mz_sql_parser::ast::{display::AstFormatter, Ident};
use mz_sql_parser::ast::{UnresolvedDatabaseName, UnresolvedObjectName};

use astdisplay::*;

#[derive(AstDisplay)]
enum Blah {
    Yo,
    Foo,
}

#[derive(AstDisplay)]
pub struct DropRolesStatement {
    pub if_exists: bool,
    pub names: Vec<Ident>,
}

#[derive(AstDisplay)]
struct AlterConnectionStatement {
    pub if_exists: bool,
    pub name: UnresolvedObjectName,
}

#[derive(AstDisplay)]
pub struct DiscardStatement {
    pub target: DiscardTarget,
}

#[derive(AstDisplay)]
pub enum DiscardTarget {
    Plans,
    Sequences,
    Temp,
    All,
}

#[derive(AstDisplay)]
pub struct DropDatabaseStatement {
    pub if_exists: bool,
    pub name: UnresolvedDatabaseName,
    pub restrict: bool,
}

fn main() {
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
    println!("{}", s.to_ast_string());
}
