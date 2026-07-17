// End-to-end tests: run the compiled binary with real args and assert on
// stdout/stderr and exit codes. Each test gets its own temp working directory
// because the app stores its data in ./todo/ relative to the cwd — without
// this, parallel tests would stomp on one shared file.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn todo(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn add_then_list_shows_task() {
    let dir = TempDir::new().unwrap();

    todo(&dir)
        .args(["add", "buy milk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("New task added!"));

    todo(&dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("1. [ ] buy milk"));
}

#[test]
fn complete_marks_task_done() {
    let dir = TempDir::new().unwrap();

    todo(&dir).args(["add", "buy milk"]).assert().success();
    todo(&dir).args(["complete", "1"]).assert().success();

    todo(&dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("1. [X] buy milk"));
}

#[test]
fn pend_reverts_completed_task() {
    let dir = TempDir::new().unwrap();

    todo(&dir).args(["add", "buy milk"]).assert().success();
    todo(&dir).args(["complete", "1"]).assert().success();
    todo(&dir).args(["pend", "1"]).assert().success();

    todo(&dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("1. [ ] buy milk"));
}

#[test]
fn rename_changes_listed_title() {
    let dir = TempDir::new().unwrap();

    todo(&dir).args(["add", "buy milk"]).assert().success();
    todo(&dir)
        .args(["rename", "1", "buy oat milk"])
        .assert()
        .success();

    todo(&dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("1. [ ] buy oat milk"));
}

#[test]
fn delete_removes_task_from_list() {
    let dir = TempDir::new().unwrap();

    todo(&dir).args(["add", "buy milk"]).assert().success();
    todo(&dir).args(["add", "walk the dog"]).assert().success();
    todo(&dir).args(["delete", "1"]).assert().success();

    todo(&dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("buy milk").not())
        .stdout(predicate::str::contains("2. [ ] walk the dog"));
}

#[test]
fn unknown_command_fails_with_error() {
    let dir = TempDir::new().unwrap();

    todo(&dir)
        .arg("frobnicate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown args"));
}

#[test]
fn missing_id_fails_with_error() {
    let dir = TempDir::new().unwrap();

    todo(&dir)
        .arg("delete")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID not provided"));
}

#[test]
fn operating_on_missing_task_fails() {
    let dir = TempDir::new().unwrap();

    todo(&dir)
        .args(["complete", "42"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task 42 not found!"));
}
