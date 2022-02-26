use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn bag_cli_tests() {
    // have to setup the manifest-encoding test here because git for windows hates newlines in paths
    setup_encoding_test();

    trycmd::TestCases::new().case("tests/cmd/bag/*.toml");
}

#[test]
fn rebag_cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/rebag/*.toml");
}

fn setup_encoding_test() {
    let in_base = base_path().join("bag").join("manifest-encoding.in");
    setup_encoding_files(in_base);

    let out_base = base_path()
        .join("bag")
        .join("manifest-encoding.out")
        .join("data");
    setup_encoding_files(out_base);
}

fn setup_encoding_files(path: PathBuf) {
    write_file(
        &path.join("dir\r\nwith%25everything%0D%0A").join("file.txt"),
        "complex name\n",
    );
    write_file(&path.join("test\nlf.txt"), "file with lf\n");
    write_file(&path.join("test\rcr.txt"), "file with cr\n");
    write_file(&path.join("test%20file.txt"), "file with %\n");
}

fn base_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("cmd");
    path
}

fn create_dir_all(path: &Path) {
    fs::create_dir_all(path).unwrap()
}

fn write_file(path: &Path, content: &str) {
    create_dir_all(path.parent().unwrap());
    fs::write(path, content).unwrap();
}
