#[test]
fn bag_cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/bag/*.toml");
}
