use std::fmt;

use astdisplay::*;

#[derive(AstDisplay)]
enum Blah {
    Yo,
    Foo,
}

#[derive(AstDisplay)]
pub struct DropRolesStatement {
    /// An optional `IF EXISTS` clause. (Non-standard.)
    pub if_exists: bool,
    /// One or more objects to drop. (ANSI SQL requires exactly one.)
    pub names: Vec<Ident>,
}

fn main() {
    let s = Blah::Yo;
    println!("{}", s.to_ast_string());
}
