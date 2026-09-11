//! Codegen freshness: the committed generated file must equal fresh
//! generator output over the committed IDL. Update mode rewrites it.

use std::path::PathBuf;

use nbe_bindings::gen::generate_interfaces;
use nbe_bindings::idl::parse_idl;

fn dom_webidl_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../nbe-dom/idl/dom.webidl")
}

fn generated_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../nbe-dom/src/interfaces.rs")
}

#[test]
fn generated_interfaces_file_is_current() {
    let idl = std::fs::read_to_string(dom_webidl_path()).unwrap();
    let interfaces = parse_idl(&idl).unwrap();
    let fresh = generate_interfaces(&interfaces);
    let update = std::env::var("NBE_UPDATE_GOLDENS")
        .map(|v| v == "1")
        .unwrap_or(false);
    if update {
        std::fs::write(generated_path(), &fresh).unwrap();
    }
    let committed = std::fs::read_to_string(generated_path()).unwrap();
    assert_eq!(
        committed, fresh,
        "generated file is stale: regenerate with NBE_UPDATE_GOLDENS=1, then commit"
    );
}

#[test]
fn webidl_file_parses_to_expected_interfaces() {
    let idl = std::fs::read_to_string(dom_webidl_path()).unwrap();
    let interfaces = parse_idl(&idl).unwrap();
    let names: Vec<&str> = interfaces.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["Node", "Element", "CodegenProbe"]);
}
