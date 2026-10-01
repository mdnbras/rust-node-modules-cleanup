use rust_node_modules_cleanup::{
    cli::{self, Command},
    format_size, scan,
};
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rust-nmc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, path: &str, data: &[u8]) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, data).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command(path: &Path) -> Process {
    let mut command = Process::new(env!("CARGO_BIN_EXE_rust-node-modules-cleanup"));
    command.arg(path);
    command
}

#[test]
fn finds_top_level_dependencies_and_counts_nested_and_hidden_files() {
    let fixture = Fixture::new();
    fixture.file("a/node_modules/pkg/index.js", b"1234");
    fixture.file("a/node_modules/pkg/node_modules/dep/index.js", b"12");
    fixture.file("a/node_modules/.cache/data", b"123");
    fixture.file("b/node_modules/pkg/index.js", b"1");
    fixture.file(".hidden/node_modules/pkg/index.js", b"12345");
    fixture.file("node_modules_backup/index.js", b"12345");
    let result = scan::scan(&fixture.0, 2).unwrap();
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(result.total_bytes(), 10);
    assert!(result.warnings.is_empty());
}

#[test]
fn direct_node_modules_path_is_supported() {
    let fixture = Fixture::new();
    fixture.file("node_modules/pkg.js", b"123");
    let result = scan::scan(&fixture.0.join("node_modules"), 1).unwrap();
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.total_bytes(), 3);
}

#[test]
fn rejects_invalid_roots_and_worker_counts() {
    let fixture = Fixture::new();
    let file = fixture.file("file", b"x");
    assert!(scan::scan(&file, 1).is_err());
    assert!(scan::scan(&fixture.0.join("missing"), 1).is_err());
    assert!(scan::scan(&fixture.0, 0).is_err());
    assert!(scan::scan(&fixture.0, 65).is_err());
}

#[test]
fn dry_run_with_yes_never_deletes_or_prompts() {
    let fixture = Fixture::new();
    let file = fixture.file("app/node_modules/pkg.js", b"123");
    let output = command(&fixture.0)
        .args(["--dry", "-y"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(file.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("nothing was deleted"));
    assert!(!stdout.contains("Permanently delete"));
}

#[test]
fn eof_and_negative_confirmation_preserve_files() {
    let fixture = Fixture::new();
    let file = fixture.file("node_modules/a", b"x");
    let output = command(&fixture.0).stdin(Stdio::null()).output().unwrap();
    assert!(output.status.success());
    assert!(file.exists());
    let mut child = command(&fixture.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"no\n").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(file.exists());
}

#[test]
fn positive_confirmation_deletes_only_dependencies() {
    let fixture = Fixture::new();
    let file = fixture.file("app/node_modules/pkg.js", b"123");
    let keep = fixture.file("app/package.json", b"{}");
    let mut child = command(&fixture.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"yes\n").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(!file.exists());
    assert!(keep.exists());
}

#[test]
fn skip_confirmation_deletes_multiple_projects() {
    let fixture = Fixture::new();
    let a = fixture.file("a/node_modules/a", b"x");
    let b = fixture.file("b/node_modules/b", b"y");
    let output = command(&fixture.0)
        .args(["--skip-confirmation", "--jobs", "2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!a.exists() && !b.exists());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Deleted: 2; failed: 0"));
}

#[test]
fn errors_return_nonzero_and_empty_search_succeeds() {
    let fixture = Fixture::new();
    assert!(command(&fixture.0)
        .arg("--dry")
        .output()
        .unwrap()
        .status
        .success());
    assert!(!command(&fixture.0.join("missing"))
        .arg("-y")
        .output()
        .unwrap()
        .status
        .success());
    assert!(!command(&fixture.0)
        .arg("--typo")
        .output()
        .unwrap()
        .status
        .success());
}

#[test]
fn removed_candidate_is_not_reported_as_deleted() {
    let fixture = Fixture::new();
    fixture.file("node_modules/file", b"x");
    let scan = scan::scan(&fixture.0, 1).unwrap();
    fs::remove_dir_all(&scan.candidates[0].path).unwrap();
    assert!(scan.delete(&scan.candidates[0]).is_err());
}

#[cfg(unix)]
#[test]
fn skips_symlinks_and_preserves_external_targets_during_deletion() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let keep = outside.file("node_modules/keep", b"external");
    fixture.file("app/node_modules/pkg", b"123");
    symlink(&outside.0, fixture.0.join("linked-project")).unwrap();
    symlink(&outside.0, fixture.0.join("app/node_modules/link")).unwrap();
    symlink(
        outside.0.join("missing"),
        fixture.0.join("app/node_modules/broken"),
    )
    .unwrap();
    symlink(&outside.0, fixture.0.join("node_modules")).unwrap();
    let scan = scan::scan(&fixture.0, 1).unwrap();
    assert_eq!(scan.candidates.len(), 1);
    assert_eq!(scan.total_bytes(), 3);
    scan.delete(&scan.candidates[0]).unwrap();
    assert!(keep.exists());
    assert!(scan::scan(&fixture.0.join("node_modules"), 1).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_replaced_directory_and_changed_ancestor() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.file("app/node_modules/pkg", b"123");
    let scan = scan::scan(&fixture.0, 1).unwrap();
    let candidate = &scan.candidates[0];
    fs::rename(&candidate.path, fixture.0.join("saved")).unwrap();
    fs::create_dir(&candidate.path).unwrap();
    assert!(scan.delete(candidate).is_err());
    fs::remove_dir(&candidate.path).unwrap();
    fs::rename(fixture.0.join("saved"), &candidate.path).unwrap();
    fs::rename(fixture.0.join("app"), fixture.0.join("moved")).unwrap();
    symlink(fixture.0.join("moved"), fixture.0.join("app")).unwrap();
    assert!(scan.delete(candidate).is_err());
    assert!(fixture.0.join("moved/node_modules/pkg").exists());
}

#[cfg(unix)]
#[test]
fn handles_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    let project = fixture.0.join(OsString::from_vec(vec![b'p', 0xff]));
    fs::create_dir_all(project.join("node_modules")).unwrap();
    assert!(command(&project)
        .arg("--dry")
        .output()
        .unwrap()
        .status
        .success());
}

#[test]
fn parses_aliases_and_options_before_path() {
    let args = ["--dry-run", "-y", "--jobs", "3", "a b"].map(OsString::from);
    let Command::Run(options) = cli::parse(args).unwrap() else {
        panic!()
    };
    assert!(options.dry && options.yes);
    assert_eq!(options.jobs, 3);
    assert_eq!(options.path, PathBuf::from("a b"));
    assert_eq!(cli::parse(["--h".into()]).unwrap(), Command::Help);
    assert_eq!(cli::parse(["--v".into()]).unwrap(), Command::Version);
    assert!(cli::parse(["--".into(), "-project".into()]).is_ok());
    for args in [
        vec![],
        vec!["a", "b"],
        vec!["a", "--jobs", "0"],
        vec!["a", "--jobs", "65"],
        vec!["a", "--jobs"],
        vec!["a", "--unknown"],
    ] {
        assert!(cli::parse(args.into_iter().map(OsString::from)).is_err());
    }
}

#[test]
fn formatting_and_confirmation() {
    assert_eq!(format_size(0), "0.00 B");
    assert_eq!(format_size(1024), "1.00 KiB");
    assert_eq!(format_size(1_048_576), "1.00 MiB");
    for answer in ["yes", "Y", " SIM \n", "kör bara kör!"] {
        assert!(cli::confirmed(answer));
    }
    for answer in ["", "no", "maybe"] {
        assert!(!cli::confirmed(answer));
    }
}
