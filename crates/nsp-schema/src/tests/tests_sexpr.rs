//! S-expression parsing tests.

use crate::{parse_schematic, parse_sexpr, parse_symbol_library, parse_symbol_library_table};

#[test]
fn parses_quoted_strings_and_comments() {
    let parsed = parse_sexpr("(root ; comment\n  \"quoted value\" (child \"a\\\\b\"))").unwrap();
    let items = match parsed {
        crate::sexpr::Sexp::List(items) => items,
        crate::sexpr::Sexp::Atom(_) => panic!("root should be a list"),
    };
    assert_eq!(items.len(), 3);
}

#[test]
fn rejects_wrong_schema_root() {
    let error = parse_schematic("(nsp_symbol_lib)", "bad.nsp_sch").unwrap_err();
    assert!(
        error.to_string().contains("expected one of") || error.to_string().contains("expected"),
        "unexpected error message: {}",
        error
    );

    let error = parse_symbol_library("(nsp_sch)", "bad.nsp_sym").unwrap_err();
    assert!(
        error.to_string().contains("expected one of") || error.to_string().contains("expected"),
        "unexpected error message: {}",
        error
    );

    let error = parse_symbol_library_table("(nsp_sch)", "sym-lib-table").unwrap_err();
    assert!(
        error.to_string().contains("expected one of") || error.to_string().contains("expected"),
        "unexpected error message: {}",
        error
    );
}
