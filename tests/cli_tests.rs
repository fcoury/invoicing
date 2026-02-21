use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn invoice_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("invoice"))
}

#[test]
fn test_help() {
    invoice_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Minimal CLI invoicing system"));
}

#[test]
fn test_version() {
    invoice_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("invoice"));
}

#[test]
fn test_init_creates_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized invoice config"));

    // Check files were created
    assert!(config_path.join("config.toml").exists());
    assert!(config_path.join("clients.toml").exists());
    assert!(config_path.join("items.toml").exists());
}

#[test]
fn test_init_fails_if_exists() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // First init should succeed
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Second init should fail
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_status_without_init() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("nonexistent");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_clients_list() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // List clients
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "clients"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example-client"))
        .stdout(predicate::str::contains("Example Client Inc."));
}

#[test]
fn test_items_list() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // List items
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "items"])
        .assert()
        .success()
        .stdout(predicate::str::contains("consulting"))
        .stdout(predicate::str::contains("Technical Consulting"))
        .stdout(predicate::str::contains("150.00"));
}

#[test]
fn test_status() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Check status
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invoice Status"))
        .stdout(predicate::str::contains("Next invoice:"))
        .stdout(predicate::str::contains("INV-"));
}

#[test]
fn test_generate_missing_client() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Try to generate with non-existent client
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "generate",
            "--client",
            "nonexistent",
            "--item",
            "consulting:8",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Client 'nonexistent' not found"));
}

#[test]
fn test_generate_missing_item() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Try to generate with non-existent item
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "generate",
            "--client",
            "example-client",
            "--item",
            "nonexistent:8",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Item 'nonexistent' not found"));
}

#[test]
fn test_generate_invalid_quantity() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Try to generate with invalid quantity
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "generate",
            "--client",
            "example-client",
            "--item",
            "consulting:abc",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid quantity"));
}

#[test]
fn test_generate_no_items() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    // Initialize
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Try to generate without items
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "generate",
            "--client",
            "example-client",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No items specified"));
}

fn write_state(config_path: &std::path::Path, state: &str) {
    fs::write(config_path.join("state.toml"), state).unwrap();
}

#[test]
fn test_mark_paid_by_number_and_list_status() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    write_state(
        &config_path,
        r#"[counter]
last_number = 2
last_year = 2026

[[history]]
number = "INV-2026-0001"
client = "example-client"
date = "2026-01-10"
total = 1000.0
file = "INV-2026-0001.pdf"
items = ["consulting:4"]
paid = false

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-11"
total = 500.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]
paid = false
"#,
    );

    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "mark-paid",
            "INV-2026-0001",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked INV-2026-0001 as paid"));

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("STATUS"))
        .stdout(predicate::str::contains("PAID"))
        .stdout(predicate::str::contains("UNPAID"));
}

#[test]
fn test_mark_unpaid_by_index_and_limit_totals_scope() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    write_state(
        &config_path,
        r#"[counter]
last_number = 3
last_year = 2026

[[history]]
number = "INV-2026-0001"
client = "example-client"
date = "2026-01-10"
total = 100.0
file = "INV-2026-0001.pdf"
items = ["consulting:1"]
paid = false

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-11"
total = 200.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]
paid = true

[[history]]
number = "INV-2026-0003"
client = "example-client"
date = "2026-01-12"
total = 300.0
file = "INV-2026-0003.pdf"
items = ["consulting:3"]
paid = true
"#,
    );

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "mark-unpaid", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked INV-2026-0003 as unpaid"));

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list", "--limit", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TOTAL"))
        .stdout(predicate::str::contains("(-) PAID"))
        .stdout(predicate::str::contains("(=) OUTSTANDING"))
        .stdout(predicate::str::contains("$   500"))
        .stdout(predicate::str::contains("$   200"))
        .stdout(predicate::str::contains("$   300"));
}

#[test]
fn test_list_legacy_entries_default_to_unpaid() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    write_state(
        &config_path,
        r#"[counter]
last_number = 1
last_year = 2026

[[history]]
number = "INV-2026-0001"
client = "example-client"
date = "2026-01-10"
total = 1250.0
file = "INV-2026-0001.pdf"
items = ["consulting:5"]
"#,
    );

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("UNPAID"))
        .stdout(predicate::str::contains("(=) OUTSTANDING"))
        .stdout(predicate::str::contains("$ 1,250"));
}
