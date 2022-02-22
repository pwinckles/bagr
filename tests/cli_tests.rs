#[test]
fn bag_cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/bag/*.toml");
}

#[test]
fn rebag_cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/rebag/*.toml");
}
