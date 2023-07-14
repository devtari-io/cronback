#[test]
fn dto_tests() {
    let t = trybuild::TestCases::new();
    t.pass("./tests/simple-into-proto-enum.rs");
    t.pass("./tests/into-proto-enum-unit.rs");
    t.pass("./tests/simple-into-proto-struct.rs");
}
