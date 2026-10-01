use std::path::PathBuf;
use std::process::Command;

/// Resolves from the running test artifact so a relocated target directory
/// cannot leave Cargo's compile-time absolute binary path embedded in tests.
pub fn gateway_command() -> Command {
    Command::new(profile_binary("zixcel-mcp-gateway"))
}

/// Cargo runs package tests from the package root; checking the marker keeps
/// fixture access explicit and avoids embedding the checkout's absolute path.
pub fn fixture(relative: &str) -> PathBuf {
    let root = std::env::current_dir().expect("test working directory");
    assert!(
        root.join("Cargo.toml").is_file(),
        "tests must run from the Cargo package root"
    );
    let path = root.join(relative);
    assert!(path.is_file(), "missing test fixture: {}", path.display());
    path
}

fn profile_binary(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("current test executable");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(path.is_file(), "missing Cargo binary: {}", path.display());
    path
}
