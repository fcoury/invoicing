# Client Invoice Report PDF — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an `invoice report` command that generates a professional PDF statement listing all invoices for a given client, with optional filters.

**Architecture:** New Typst template for the report, a `ReportData` struct serialized to JSON, and a `generate_report_pdf()` function following the existing `generate_pdf()` pattern. New `Commands::Report` clap variant and `cmd_report()` handler.

**Tech Stack:** Rust, clap, serde/serde_json, chrono, Typst CLI

---

### Task 1: Add ReportData struct and ReportInvoiceRow

**Files:**
- Create: `src/invoice/report.rs`
- Modify: `src/invoice/mod.rs` (add `mod report; pub use report::*;`)

**Step 1: Create `src/invoice/report.rs` with data structs**

```rust
use serde::Serialize;

use crate::config::{Client, Company};

/// A single invoice row in the report
#[derive(Debug, Serialize)]
pub struct ReportInvoiceRow {
    pub number: String,
    pub date: String,
    pub total: f64,
    pub status: String,
}

/// Complete report data for PDF generation
#[derive(Debug, Serialize)]
pub struct ReportData {
    pub company: Company,
    pub client: Client,
    pub invoices: Vec<ReportInvoiceRow>,
    pub total: f64,
    pub paid: f64,
    pub outstanding: f64,
    pub currency_symbol: String,
    pub generated_date: String,
    pub filter_from: Option<String>,
    pub filter_to: Option<String>,
    pub filter_status: Option<String>,
}
```

**Step 2: Wire up the module**

In `src/invoice/mod.rs`, add:
```rust
mod report;
pub use report::{ReportData, ReportInvoiceRow};
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: compiles cleanly

**Step 4: Commit**

```
feat(report): add ReportData and ReportInvoiceRow structs
```

---

### Task 2: Add Typst report template and generate_report_pdf()

**Files:**
- Modify: `src/pdf/typst.rs` (add `REPORT_TEMPLATE` const and `generate_report_pdf`)
- Modify: `src/pdf/mod.rs` (re-export `generate_report_pdf`)

**Step 1: Add `REPORT_TEMPLATE` const to `src/pdf/typst.rs`**

Add after the existing `INVOICE_TEMPLATE` const (line 148). The template should:
- Reuse the same `fmt-int` and `fmt-currency` helper functions from the invoice template
- Show company letterhead (name, address, city/state/zip, email, phone)
- Horizontal rule separator
- Title "INVOICE REPORT" in large bold text
- Client info block (name, contact, address)
- Period/filter info line (e.g., "Period: Jan 30, 2026 – Feb 20, 2026" or "All invoices") and "Generated: Feb 21, 2026"
- Table with columns: NUMBER, DATE, TOTAL, STATUS
- Financial summary aligned right: Total, (-) Paid, (=) Outstanding

```typst
const REPORT_TEMPLATE: &str = r##"// Report Template
#let data = json("DATA_JSON_PATH")

#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
)

#set text(font: "Helvetica", size: 10pt)

#let fmt-int(digits) = {
  let len = digits.len()
  let out = ""
  for (i, digit) in digits.clusters().enumerate() {
    if i > 0 and calc.rem(len - i, 3) == 0 {
      out += ","
    }
    out += digit
  }
  out
}

#let fmt-currency(amount) = {
  let parts = str(calc.round(amount, digits: 2)).split(".")
  let whole = fmt-int(parts.at(0))
  let frac = if parts.len() > 1 { parts.at(1) } else { "00" }
  let frac2 = if frac.len() == 1 { frac + "0" } else { frac }
  data.currency_symbol + whole + "." + frac2
}

// Company letterhead
#grid(
  columns: (1fr, 1fr),
  align: (left, right),
  [
    #text(size: 18pt, weight: "bold")[#data.company.name]
    #v(0.3em)
    #data.company.address \
    #data.company.city, #data.company.state #data.company.zip \
    #data.company.email
    #if data.company.phone != none [
      \ #data.company.phone
    ]
  ],
  [
    #text(size: 24pt, weight: "bold")[INVOICE REPORT]
    #v(0.5em)
    #text(size: 10pt)[Generated: #data.generated_date]
  ]
)

#v(1em)
#line(length: 100%, stroke: 0.5pt + gray)
#v(1em)

// Client info
#grid(
  columns: (1fr, 1fr),
  [
    #text(weight: "bold", size: 11pt)[Client:]
    #v(0.3em)
    #text(weight: "bold")[#data.client.name]
    #if data.client.contact != none [
      \ #data.client.contact
    ]
    \ #data.client.address
    \ #data.client.city, #data.client.state #data.client.zip
    \ #data.client.email
  ],
  align(right)[
    // Filter info
    #if data.filter_from != none and data.filter_to != none [
      Period: #data.filter_from – #data.filter_to
    ] else if data.filter_from != none [
      From: #data.filter_from
    ] else if data.filter_to != none [
      Through: #data.filter_to
    ] else [
      All invoices
    ]
    #if data.filter_status != none [
      \ Status: #data.filter_status
    ]
  ]
)

#v(1.5em)

// Invoice table
#table(
  columns: (auto, auto, auto, auto),
  align: (left, left, right, center),
  stroke: (x, y) => if y == 0 { (bottom: 1pt + black) } else if y > 0 { (bottom: 0.5pt + gray) },
  inset: 8pt,
  fill: (x, y) => if y == 0 { luma(240) } else { none },

  [*Number*], [*Date*], [*Total*], [*Status*],

  ..data.invoices.map(inv => (
    inv.number,
    inv.date,
    [#fmt-currency(inv.total)],
    inv.status,
  )).flatten()
)

#v(1em)

// Financial summary
#align(right)[
  #table(
    columns: (auto, auto),
    stroke: none,
    align: (right, right),
    inset: 6pt,

    [Subtotal:], [#fmt-currency(data.total)],
    [Paid:], [#fmt-currency(data.paid)],
    table.hline(stroke: 1pt),
    [*Outstanding:*], [*#fmt-currency(data.outstanding)*],
  )
]

#if data.company.tax_id != none [
  #v(2em)
  #text(size: 9pt, fill: gray)[Tax ID: #data.company.tax_id]
]
"##;
```

**Step 2: Add `generate_report_pdf` function after existing `generate_pdf`**

```rust
use crate::invoice::ReportData;

pub fn generate_report_pdf(report_data: &ReportData, output_path: &PathBuf) -> Result<()> {
    let typst_check = Command::new("typst").arg("--version").output();
    if typst_check.is_err() {
        return Err(InvoiceError::TypstNotFound);
    }

    let temp_dir = std::env::temp_dir().join("invoice-cli");
    std::fs::create_dir_all(&temp_dir)?;

    let json_data = serde_json::to_string(report_data)
        .map_err(|e| InvoiceError::PdfGeneration(e.to_string()))?;

    let json_path = temp_dir.join("data.json");
    std::fs::write(&json_path, &json_data)?;

    let template_content = REPORT_TEMPLATE.replace("DATA_JSON_PATH", "data.json");
    let template_path = temp_dir.join("report.typ");
    std::fs::write(&template_path, &template_content)?;

    let output = Command::new("typst")
        .args([
            "compile",
            "--root",
            temp_dir.to_str().unwrap(),
            template_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(InvoiceError::PdfGeneration(stderr.to_string()));
    }

    let _ = std::fs::remove_file(&template_path);
    let _ = std::fs::remove_file(&json_path);

    Ok(())
}
```

**Step 3: Re-export from `src/pdf/mod.rs`**

Change to:
```rust
mod typst;

pub use typst::{generate_pdf, generate_report_pdf};
```

**Step 4: Update `src/invoice/report.rs` import in `src/pdf/typst.rs`**

Add `use crate::invoice::ReportData;` at top of file alongside existing `use crate::invoice::InvoiceData;`.

**Step 5: Build and verify**

Run: `cargo build`
Expected: compiles cleanly

**Step 6: Commit**

```
feat(report): add Typst report template and generate_report_pdf
```

---

### Task 3: Add CLI command and cmd_report handler

**Files:**
- Modify: `src/main.rs` (add `Commands::Report` variant and `cmd_report` function)

**Step 1: Add `Commands::Report` variant**

After the `MarkUnpaid` variant in the `Commands` enum (around line 108), add:

```rust
/// Generate a PDF report of invoices for a client
Report {
    /// Client identifier from clients.toml
    #[arg(short, long)]
    client: String,

    /// Filter: start date (inclusive, YYYY-MM-DD)
    #[arg(long)]
    from: Option<String>,

    /// Filter: end date (inclusive, YYYY-MM-DD)
    #[arg(long)]
    to: Option<String>,

    /// Filter: payment status (paid or unpaid)
    #[arg(long)]
    status: Option<String>,

    /// Open generated PDF with system default viewer
    #[arg(long)]
    open: bool,
},
```

**Step 2: Add match arm in `run()`**

After the `MarkUnpaid` match arm (around line 144), add:

```rust
Commands::Report { client, from, to, status, open } => {
    cmd_report(&cfg_dir, &client, from.as_deref(), to.as_deref(), status.as_deref(), open)
}
```

**Step 3: Add imports**

At top of file, update:
```rust
use crate::invoice::{generate_invoice, get_invoice_path, regenerate_invoice};
```
to:
```rust
use crate::invoice::{generate_invoice, get_invoice_path, regenerate_invoice, ReportData, ReportInvoiceRow};
```

And add:
```rust
use crate::pdf::generate_report_pdf;
```

Also add `use chrono::NaiveDate;` if not already imported.

**Step 4: Add `cmd_report` function**

Add before or after `cmd_invoices`:

```rust
fn cmd_report(
    cfg_dir: &PathBuf,
    client_id: &str,
    from: Option<&str>,
    to: Option<&str>,
    status: Option<&str>,
    open: bool,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let config = load_config(cfg_dir)?;
    let clients = load_clients(cfg_dir)?;
    let state = load_state(cfg_dir)?;

    // Validate client exists
    let client = clients
        .get(client_id)
        .ok_or_else(|| InvoiceError::ClientNotFound(client_id.to_string()))?
        .clone();

    // Parse optional date filters
    let from_date = from
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| InvoiceError::PdfGeneration("Invalid --from date, expected YYYY-MM-DD".to_string()))?;
    let to_date = to
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| InvoiceError::PdfGeneration("Invalid --to date, expected YYYY-MM-DD".to_string()))?;

    // Validate --status flag
    if let Some(s) = status {
        if s != "paid" && s != "unpaid" {
            return Err(InvoiceError::PdfGeneration(
                "Invalid --status, expected 'paid' or 'unpaid'".to_string(),
            ));
        }
    }

    // Filter invoices for this client
    let filtered: Vec<_> = state
        .history
        .iter()
        .filter(|e| e.client == client_id)
        .filter(|e| from_date.map_or(true, |d| e.date >= d))
        .filter(|e| to_date.map_or(true, |d| e.date <= d))
        .filter(|e| match status {
            Some("paid") => e.paid,
            Some("unpaid") => !e.paid,
            _ => true,
        })
        .collect();

    if filtered.is_empty() {
        println!("No invoices found for client '{}' with the given filters.", client_id);
        return Ok(());
    }

    // Build report rows
    let invoice_rows: Vec<ReportInvoiceRow> = filtered
        .iter()
        .rev()
        .map(|e| ReportInvoiceRow {
            number: e.number.clone(),
            date: e.date.format("%B %d, %Y").to_string(),
            total: e.total,
            status: if e.paid { "Paid".to_string() } else { "Unpaid".to_string() },
        })
        .collect();

    let total: f64 = filtered.iter().map(|e| e.total).sum();
    let paid: f64 = filtered.iter().filter(|e| e.paid).map(|e| e.total).sum();
    let outstanding: f64 = filtered.iter().filter(|e| !e.paid).map(|e| e.total).sum();

    let today = chrono::Local::now().format("%B %d, %Y").to_string();

    let report_data = ReportData {
        company: config.company.clone(),
        client,
        invoices: invoice_rows,
        total,
        paid,
        outstanding,
        currency_symbol: config.invoice.currency_symbol.clone(),
        generated_date: today,
        filter_from: from.map(String::from),
        filter_to: to.map(String::from),
        filter_status: status.map(String::from),
    };

    // Determine output path
    let output_dir = crate::config::resolve_output_dir(&config.pdf.output_dir, cfg_dir);
    std::fs::create_dir_all(&output_dir)?;

    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let filename = format!("REPORT-{}-{}.pdf", client_id, date_str);
    let pdf_path = output_dir.join(&filename);

    generate_report_pdf(&report_data, &pdf_path)?;

    println!("Generated report: {}", filename);
    println!("  Client:   {}", report_data.company.name);
    println!("  Invoices: {}", filtered.len());
    println!("  Saved:    {}", pdf_path.display());

    if open {
        open_path(&pdf_path)?;
    }

    Ok(())
}
```

**Step 5: Build and verify**

Run: `cargo build`
Expected: compiles cleanly

**Step 6: Commit**

```
feat(report): add report command with date and status filters
```

---

### Task 4: Write CLI tests for the report command

**Files:**
- Modify: `tests/cli_tests.rs`

**Step 1: Add test for report with unknown client**

```rust
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
            "-C", config_path.to_str().unwrap(),
            "report", "--client", "nonexistent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Client 'nonexistent' not found"));
}
```

**Step 2: Add test for report with no matching invoices**

```rust
#[test]
fn test_report_no_invoices_for_client() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    invoice_cmd()
        .args([
            "-C", config_path.to_str().unwrap(),
            "report", "--client", "example-client",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No invoices found"));
}
```

**Step 3: Add test for successful report generation (requires typst)**

```rust
#[test]
fn test_report_generates_pdf() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    // Create output directory
    fs::create_dir_all(config_path.join("output")).unwrap();

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
paid = true

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-20"
total = 500.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]
paid = false
"#,
    );

    invoice_cmd()
        .args([
            "-C", config_path.to_str().unwrap(),
            "report", "--client", "example-client",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated report"))
        .stdout(predicate::str::contains("REPORT-example-client-"))
        .stdout(predicate::str::contains("Invoices: 2"));

    // Verify PDF was created
    let output_dir = config_path.join("output");
    let pdfs: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("REPORT-"))
        .collect();
    assert_eq!(pdfs.len(), 1);
}
```

**Step 4: Add test for --status filter**

```rust
#[test]
fn test_report_status_filter() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invoice-config");

    invoice_cmd()
        .args(["-C", config_path.to_str().unwrap(), "init"])
        .assert()
        .success();

    fs::create_dir_all(config_path.join("output")).unwrap();

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
paid = true

[[history]]
number = "INV-2026-0002"
client = "example-client"
date = "2026-01-20"
total = 500.0
file = "INV-2026-0002.pdf"
items = ["consulting:2"]
paid = false
"#,
    );

    // Only paid invoices
    invoice_cmd()
        .args([
            "-C", config_path.to_str().unwrap(),
            "report", "--client", "example-client", "--status", "paid",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invoices: 1"));
}
```

**Step 5: Run tests**

Run: `cargo test`
Expected: all tests pass (tests requiring typst will pass if typst is installed)

**Step 6: Commit**

```
test(report): add CLI tests for report command
```

---

### Task 5: Manual verification and final commit

**Step 1: Run the actual command against your real data**

```bash
cargo run -- report --client openai
```

Expected: generates `REPORT-openai-2026-02-21.pdf` in output directory, prints summary

**Step 2: Open and visually verify**

```bash
cargo run -- report --client openai --open
```

Verify: PDF shows company letterhead, client info, invoice table, financial summary

**Step 3: Test filters**

```bash
cargo run -- report --client openai --status paid
cargo run -- report --client openai --from 2026-02-01 --to 2026-02-28
```

**Step 4: Run full test suite**

Run: `cargo test`
Expected: all tests pass

**Step 5: Final commit if any fixes were needed**

```
fix(report): polish report template after manual verification
```
