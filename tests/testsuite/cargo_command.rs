//! Tests for custom cargo commands and other global command features.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str;

use cargo_test_support::cargo_process;
use cargo_test_support::paths;
use cargo_test_support::registry::Package;
use cargo_test_support::{basic_bin_manifest, basic_manifest, cargo_exe, project, project_in_home};

fn path() -> Vec<PathBuf> {
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect()
}

#[cargo_test]
fn list_commands_with_descriptions() {
    let p = project().build();
    p.cargo("--list")
        .with_stdout_contains(
            "    build                Compile a local package and all of its dependencies",
        )
        // Assert that `read-manifest` prints the right one-line description followed by another
        // command, indented.
        .with_stdout_contains(
            "    read-manifest        Print a JSON representation of a Cargo.toml manifest.",
        )
        .run();
}

#[cargo_test]
fn list_builtin_aliases_with_descriptions() {
    let p = project().build();
    p.cargo("--list")
        .with_stdout_contains("    b                    alias: build")
        .with_stdout_contains("    c                    alias: check")
        .with_stdout_contains("    r                    alias: run")
        .with_stdout_contains("    t                    alias: test")
        .run();
}

#[cargo_test]
fn list_custom_aliases_with_descriptions() {
    let p = project_in_home("proj")
        .file(
            &paths::home().join(".cargo").join("config"),
            r#"
            [alias]
            myaliasstr = "foo --bar"
            myaliasvec = ["foo", "--bar"]
        "#,
        )
        .build();

    p.cargo("--list")
        .with_stdout_contains("    myaliasstr           foo --bar")
        .with_stdout_contains("    myaliasvec           foo --bar")
        .run();
}

#[cargo_test]
fn list_dedupe() {
    let p = project()
        .executable(Path::new("path-test-1").join("cargo-dupe"), "")
        .executable(Path::new("path-test-2").join("cargo-dupe"), "")
        .build();

    let mut path = path();
    path.push(p.root().join("path-test-1"));
    path.push(p.root().join("path-test-2"));
    let path = env::join_paths(path.iter()).unwrap();

    p.cargo("--list")
        .env("PATH", &path)
        .with_stdout_contains_n("    dupe", 1)
        .run();
}

#[cargo_test]
fn list_command_looks_at_path() {
    let proj = project()
        .executable(Path::new("path-test").join("cargo-1"), "")
        .build();

    let mut path = path();
    path.push(proj.root().join("path-test"));
    let path = env::join_paths(path.iter()).unwrap();
    let output = cargo_process("-v --list")
        .env("PATH", &path)
        .exec_with_output()
        .unwrap();
    let output = str::from_utf8(&output.stdout).unwrap();
    assert!(
        output.contains("\n    1                   "),
        "missing 1: {}",
        output
    );
}

#[cargo_test]
fn list_command_resolves_symlinks() {
    let proj = project()
        .symlink(cargo_exe(), Path::new("path-test").join("cargo-2"))
        .build();

    let mut path = path();
    path.push(proj.root().join("path-test"));
    let path = env::join_paths(path.iter()).unwrap();
    let output = cargo_process("-v --list")
        .env("PATH", &path)
        .exec_with_output()
        .unwrap();
    let output = str::from_utf8(&output.stdout).unwrap();
    assert!(
        output.contains("\n    2                   "),
        "missing 2: {}",
        output
    );
}

#[cargo_test]
fn find_closest_biuld_to_build() {
    cargo_process("biuld")
        .with_status(101)
        .with_stderr_contains(
            "\
error: no such subcommand: `biuld`

<tab>Did you mean `build`?
",
        )
        .run();

    // But, if we actually have `biuld`, it must work!
    // https://github.com/rust-lang/cargo/issues/5201
    Package::new("cargo-biuld", "1.0.0")
        .file(
            "src/main.rs",
            r#"
                fn main() {
                    println!("Similar, but not identical to, build");
                }
            "#,
        )
        .publish();

    cargo_process("install cargo-biuld").run();
    cargo_process("biuld")
        .with_stdout("Similar, but not identical to, build\n")
        .run();
    cargo_process("--list")
        .with_stdout_contains(
            "    build                Compile a local package and all of its dependencies\n",
        )
        .with_stdout_contains("    biuld\n")
        .run();
}

#[cargo_test]
fn find_closest_alias() {
    let root = paths::root();
    let my_home = root.join("my_home");
    fs::create_dir(&my_home).unwrap();
    fs::write(
        &my_home.join("config"),
        r#"
            [alias]
            myalias = "build"
        "#,
    )
    .unwrap();

    cargo_process("myalais")
        .env("CARGO_HOME", &my_home)
        .with_status(101)
        .with_stderr_contains(
            "\
error: no such subcommand: `myalais`

<tab>Did you mean `myalias`?
",
        )
        .run();

    // But, if no alias is defined, it must not suggest one!
    cargo_process("myalais")
        .with_status(101)
        .with_stderr_contains(
            "\
error: no such subcommand: `myalais`
",
        )
        .with_stderr_does_not_contain(
            "\
<tab>Did you mean `myalias`?
",
        )
        .run();
}

// If a subcommand is more than an edit distance of 3 away, we don't make a suggestion.
#[cargo_test]
fn find_closest_dont_correct_nonsense() {
    cargo_process("there-is-no-way-that-there-is-a-command-close-to-this")
        .cwd(&paths::root())
        .with_status(101)
        .with_stderr(
            "[ERROR] no such subcommand: \
                        `there-is-no-way-that-there-is-a-command-close-to-this`
",
        )
        .run();
}

#[cargo_test]
fn displays_subcommand_on_error() {
    cargo_process("invalid-command")
        .with_status(101)
        .with_stderr("[ERROR] no such subcommand: `invalid-command`\n")
        .run();
}

#[cargo_test]
fn override_cargo_home() {
    let root = paths::root();
    let my_home = root.join("my_home");
    fs::create_dir(&my_home).unwrap();
    fs::write(
        &my_home.join("config"),
        r#"
            [cargo-new]
            vcs = "none"
        "#,
    )
    .unwrap();

    cargo_process("new foo").env("CARGO_HOME", &my_home).run();

    assert!(!paths::root().join("foo/.git").is_dir());

    cargo_process("new foo2").run();

    assert!(paths::root().join("foo2/.git").is_dir());
}

#[cargo_test]
fn cargo_subcommand_env() {
    let src = format!(
        r#"
        use std::env;

        fn main() {{
            println!("{{}}", env::var("{}").unwrap());
        }}
        "#,
        cargo::CARGO_ENV
    );

    let p = project()
        .at("cargo-envtest")
        .file("Cargo.toml", &basic_bin_manifest("cargo-envtest"))
        .file("src/main.rs", &src)
        .build();

    let target_dir = p.target_debug_dir();

    p.cargo("build").run();
    assert!(p.bin("cargo-envtest").is_file());

    let cargo = cargo_exe().canonicalize().unwrap();
    let mut path = path();
    path.push(target_dir);
    let path = env::join_paths(path.iter()).unwrap();

    cargo_process("envtest")
        .env("PATH", &path)
        .with_stdout(cargo.to_str().unwrap())
        .run();
}

#[cargo_test]
fn cargo_subcommand_args() {
    let p = project()
        .at("cargo-foo")
        .file("Cargo.toml", &basic_manifest("cargo-foo", "0.0.1"))
        .file(
            "src/main.rs",
            r#"
                fn main() {
                    let args: Vec<_> = ::std::env::args().collect();
                    println!("{}", args.join(" "));
                }
            "#,
        )
        .build();

    p.cargo("build").run();
    let cargo_foo_bin = p.bin("cargo-foo");
    assert!(cargo_foo_bin.is_file());

    let mut path = path();
    path.push(p.target_debug_dir());
    let path = env::join_paths(path.iter()).unwrap();

    cargo_process("foo bar -v --help")
        .env("PATH", &path)
        .with_stdout("[CWD]/cargo-foo/target/debug/cargo-foo[EXE] foo bar -v --help")
        .run();
}

#[cargo_test]
fn explain() {
    cargo_process("--explain E0001")
        .with_stdout_contains(
            "This error suggests that the expression arm corresponding to the noted pattern",
        )
        .run();
}

#[cargo_test]
fn closed_output_ok() {
    // Checks that closed output doesn't cause an error.
    let mut p = cargo_process("--list").build_command();
    p.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = p.spawn().unwrap();
    // Close stdout
    drop(child.stdout.take());
    // Read stderr
    let mut s = String::new();
    child
        .stderr
        .as_mut()
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(s.is_empty(), "{}", s);
}
