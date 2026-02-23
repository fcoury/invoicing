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

// ---- Payment workflow tests (replaces old mark-paid/mark-unpaid tests) ----

#[test]
fn test_add_payment_and_list_status() {
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

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-11"
total = 500.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]
"#,
    );

    // Add full payment to first invoice by number
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "add-payment",
            "INV-2026-0001",
            "1000",
            "--date",
            "2026-01-15",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded $1000.00 payment for INV-2026-0001"))
        .stdout(predicate::str::contains("fully paid"));

    // List should show PAID and UNPAID
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("STATUS"))
        .stdout(predicate::str::contains("PAID"))
        .stdout(predicate::str::contains("UNPAID"));
}

#[test]
fn test_remove_payment_and_limit_totals_scope() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Use new payments format in state
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

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-11"
total = 200.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]

[[history.payments]]
amount = 200.0
date = "2026-01-15"

[[history]]
number = "INV-2026-0003"
client = "example-client"
date = "2026-01-12"
total = 300.0
file = "INV-2026-0003.pdf"
items = ["consulting:3"]

[[history.payments]]
amount = 300.0
date = "2026-01-16"
"#,
    );

    // Remove payment from newest invoice (index 1 = INV-2026-0003)
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "remove-payment",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed $300.00 payment from INV-2026-0003"));

    // List with limit 2 should show correct totals
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
fn test_add_payment_partial_status() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]
"#,
    );

    // Add partial payment
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "add-payment",
            "1",
            "500",
            "--date",
            "2026-01-15",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded $500.00 payment for INV-2026-0001"))
        .stdout(predicate::str::contains("$700.00 remaining"));

    // List should show PARTIAL status
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PARTIAL"));
}

#[test]
fn test_add_payment_full_pays_invoice() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]
"#,
    );

    // Pay in two installments
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "add-payment",
            "1",
            "500",
        ])
        .assert()
        .success();

    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "add-payment",
            "1",
            "700",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fully paid"));

    // List should show PAID
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PAID"))
        .stdout(predicate::str::contains("UNPAID").not())
        .stdout(predicate::str::contains("PARTIAL").not());
}

#[test]
fn test_add_payment_rejects_overpayment() {
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
total = 1000.0
file = "INV-2026-0001.pdf"
items = ["consulting:4"]
"#,
    );

    // Try to overpay
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "add-payment",
            "1",
            "1500",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Payment would exceed invoice total"));
}

#[test]
fn test_remove_payment_by_index() {
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
total = 1000.0
file = "INV-2026-0001.pdf"
items = ["consulting:4"]

[[history.payments]]
amount = 300.0
date = "2026-01-15"

[[history.payments]]
amount = 400.0
date = "2026-01-20"
"#,
    );

    // Remove first payment (index 1)
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "remove-payment",
            "1",
            "--index",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed $300.00 payment from INV-2026-0001"));

    // Payments command should show only the second payment remaining
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "payments",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("400.00"))
        .stdout(predicate::str::contains("PARTIAL"));
}

#[test]
fn test_payments_shows_history() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]

[[history.payments]]
amount = 500.0
date = "2026-01-15"

[[history.payments]]
amount = 300.0
date = "2026-01-20"
"#,
    );

    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "payments",
            "INV-2026-0001",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Payments for INV-2026-0001"))
        .stdout(predicate::str::contains("500.00"))
        .stdout(predicate::str::contains("300.00"))
        .stdout(predicate::str::contains("2026-01-15"))
        .stdout(predicate::str::contains("2026-01-20"))
        .stdout(predicate::str::contains("Total paid: $800.00 / $1200.00"))
        .stdout(predicate::str::contains("PARTIAL"));
}

#[test]
fn test_legacy_paid_true_migrates_to_payment() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Old format with `paid = true`
    write_state(
        &config_path,
        r#"[counter]
last_number = 1
last_year = 2026

[[history]]
number = "INV-2026-0001"
client = "example-client"
date = "2026-01-10"
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]
paid = true
"#,
    );

    // Should show as PAID (migrated from paid=true to a full payment)
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PAID"));

    // Payments command should show the synthesized payment
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "payments",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1200.00"))
        .stdout(predicate::str::contains("2026-01-10"))
        .stdout(predicate::str::contains("PAID"));
}

#[test]
fn test_report_partial_status_filter() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]

[[history.payments]]
amount = 1200.0
date = "2026-01-15"

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-20"
total = 750.0
file = "INV-2026-0002.pdf"
items = ["consulting:5"]

[[history.payments]]
amount = 300.0
date = "2026-01-25"

[[history]]
number = "INV-2026-0003"
client = "example-client"
date = "2026-01-25"
total = 500.0
file = "INV-2026-0003.pdf"
items = ["consulting:3"]
"#,
    );

    // Filter by partial — should show only INV-2026-0002
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "report",
            "--client",
            "example-client",
            "--status",
            "partial",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invoices: 1"));
}

// ---- Existing tests that still apply ----

#[test]
fn test_report_unknown_client() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "report",
            "--client",
            "nonexistent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Client 'nonexistent' not found"));
}

#[test]
fn test_report_no_invoices_for_client() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Client exists but no invoices in history
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "report",
            "--client",
            "example-client",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No invoices found"));
}

#[test]
fn test_report_generates_pdf() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]
paid = true

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-20"
total = 750.0
file = "INV-2026-0002.pdf"
items = ["consulting:5"]
paid = false
"#,
    );

    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "report",
            "--client",
            "example-client",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated report for 'example-client'"))
        .stdout(predicate::str::contains("Invoices: 2"));

    // Verify the PDF file was created in the output directory
    let output_dir = config_path.join("output");
    let pdf_files: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map_or(false, |n| n.starts_with("REPORT-example-client-") && n.ends_with(".pdf"))
        })
        .collect();
    assert!(!pdf_files.is_empty(), "Report PDF should exist in output dir");
}

#[test]
fn test_report_status_filter() {
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
total = 1200.0
file = "INV-2026-0001.pdf"
items = ["consulting:8"]
paid = true

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-20"
total = 750.0
file = "INV-2026-0002.pdf"
items = ["consulting:5"]
paid = false
"#,
    );

    // Only paid invoices — should show 1 invoice
    invoice_cmd()
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "report",
            "--client",
            "example-client",
            "--status",
            "paid",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invoices: 1"));
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

#[test]
fn test_list_renders_core_table_regardless_of_network() {
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
total = 800.0
file = "INV-2026-0001.pdf"
items = ["consulting:4"]
paid = true

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-15"
total = 1200.0
file = "INV-2026-0002.pdf"
items = ["consulting:8"]
paid = false
"#,
    );

    // Core table elements must always render, whether or not the BRL API is reachable
    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TOTAL"))
        .stdout(predicate::str::contains("(-) PAID"))
        .stdout(predicate::str::contains("(=) OUTSTANDING"))
        .stdout(predicate::str::contains("$ 2,000"))
        .stdout(predicate::str::contains("$   800"))
        .stdout(predicate::str::contains("$ 1,200"))
        .stdout(predicate::str::contains("Total: 2 invoices"))
        .stdout(predicate::str::contains("Use index number"));
}
