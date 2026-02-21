use std::path::PathBuf;
use std::process::Command;

use crate::error::{InvoiceError, Result};
use crate::invoice::{InvoiceData, ReportData};

/// Embedded Typst template for invoice generation
/// Uses a placeholder that gets replaced with the actual JSON file path
const INVOICE_TEMPLATE: &str = r##"// Invoice Template
// Data is loaded from JSON file

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

// Header with company info and invoice details
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
    #text(size: 24pt, weight: "bold")[INVOICE]
    #v(0.5em)
    #table(
      columns: (auto, auto),
      stroke: none,
      align: (right, left),
      inset: 2pt,
      [*Invoice \#:*], [#data.number],
      [*Date:*], [#data.date],
      [*Due Date:*], [#data.due_date],
    )
  ]
)

#v(1em)
#line(length: 100%, stroke: 0.5pt + gray)
#v(1em)

// Bill To section
#grid(
  columns: (1fr, 1fr),
  [
    #text(weight: "bold", size: 11pt)[Bill To:]
    #v(0.3em)
    #text(weight: "bold")[#data.client.name]
    #if data.client.contact != none [
      \ #data.client.contact
    ]
    \ #data.client.address
    \ #data.client.city, #data.client.state #data.client.zip
    \ #data.client.email
  ],
  []
)

#v(1.5em)

// Line items table
#table(
  columns: (auto, 1fr, auto, auto, auto),
  align: (center, left, right, right, right),
  stroke: (x, y) => if y == 0 { (bottom: 1pt + black) } else if y > 0 { (bottom: 0.5pt + gray) },
  inset: 8pt,
  fill: (x, y) => if y == 0 { luma(240) } else { none },

  // Header
  [*\#*], [*Description*], [*Qty*], [*Rate*], [*Amount*],

  // Items
  ..data.items.enumerate().map(((i, item)) => (
    str(i + 1),
    item.description,
    [#item.quantity #if item.quantity == 1 { item.unit } else { item.unit + "s" }],
    [#fmt-currency(item.rate)],
    [#fmt-currency(item.amount)],
  )).flatten()
)

#v(1em)

// Totals
#align(right)[
  #table(
    columns: (auto, auto),
    stroke: none,
    align: (right, right),
    inset: 6pt,

    [Subtotal:], [#fmt-currency(data.subtotal)],

    ..if data.tax_rate > 0 {
      ([Tax (#str(calc.round(data.tax_rate, digits: 2))%):], [#fmt-currency(data.tax_amount)])
    } else {
      ()
    },

    table.hline(stroke: 1pt),
    [*Total:*], [*#fmt-currency(data.total)*],
  )
]

#v(2em)

// Payment terms (only show if due_days > 0)
#if data.due_days > 0 [
  #text(weight: "bold")[Payment Terms:] #data.payment_terms
]

#if data.company.tax_id != none [
  #v(0.5em)
  #text(size: 9pt, fill: gray)[Tax ID: #data.company.tax_id]
]
"##;

/// Generate PDF using Typst CLI
pub fn generate_pdf(invoice_data: &InvoiceData, output_path: &PathBuf) -> Result<()> {
    // Check if typst is available
    let typst_check = Command::new("typst").arg("--version").output();

    if typst_check.is_err() {
        return Err(InvoiceError::TypstNotFound);
    }

    // Create temp directory for template
    let temp_dir = std::env::temp_dir().join("invoice-cli");
    std::fs::create_dir_all(&temp_dir)?;

    // Serialize invoice data to JSON
    let json_data = serde_json::to_string(invoice_data)
        .map_err(|e| InvoiceError::PdfGeneration(e.to_string()))?;

    // Write JSON to temp file
    let json_path = temp_dir.join("data.json");
    std::fs::write(&json_path, &json_data)?;

    // Write template with relative JSON path (data.json is in same directory)
    let template_content = INVOICE_TEMPLATE.replace("DATA_JSON_PATH", "data.json");
    let template_path = temp_dir.join("invoice.typ");
    std::fs::write(&template_path, &template_content)?;

    // Run typst compile with root set to temp directory
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

    // Clean up temp files
    let _ = std::fs::remove_file(&template_path);
    let _ = std::fs::remove_file(&json_path);

    Ok(())
}

/// Embedded Typst template for invoice report generation
const REPORT_TEMPLATE: &str = r##"// Invoice Report Template
// Data is loaded from JSON file

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

// Header with company info and report title
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
    #text(size: 10pt, fill: gray)[Generated #data.generated_date]
  ]
)

#v(1em)
#line(length: 100%, stroke: 0.5pt + gray)
#v(1em)

// Client info block
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
  [
    // Filter info (right column)
    #if data.filter_from != none or data.filter_to != none or data.filter_status != none [
      #text(weight: "bold", size: 11pt)[Filters:]
      #v(0.3em)
      #if data.filter_from != none [
        From: #data.filter_from \
      ]
      #if data.filter_to != none [
        To: #data.filter_to \
      ]
      #if data.filter_status != none [
        Status: #data.filter_status
      ]
    ]
  ]
)

#v(1.5em)

// Invoice table
#table(
  columns: (auto, 1fr, auto, auto),
  align: (left, left, right, center),
  stroke: (x, y) => if y == 0 { (bottom: 1pt + black) } else if y > 0 { (bottom: 0.5pt + gray) },
  inset: 8pt,
  fill: (x, y) => if y == 0 { luma(240) } else { none },

  // Header
  [*Number*], [*Date*], [*Total*], [*Status*],

  // Rows
  ..data.rows.map(row => (
    row.number,
    row.date,
    [#fmt-currency(row.total)],
    row.status,
  )).flatten()
)

#v(1.5em)

// Financial summary (right-aligned)
#align(right)[
  #table(
    columns: (auto, auto),
    stroke: none,
    align: (right, right),
    inset: 6pt,

    [Total:], [#fmt-currency(data.total)],
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

/// Generate a report PDF using Typst CLI
pub fn generate_report_pdf(report_data: &ReportData, output_path: &PathBuf) -> Result<()> {
    // Check if typst is available
    let typst_check = Command::new("typst").arg("--version").output();

    if typst_check.is_err() {
        return Err(InvoiceError::TypstNotFound);
    }

    // Create temp directory for template
    let temp_dir = std::env::temp_dir().join("invoice-cli");
    std::fs::create_dir_all(&temp_dir)?;

    // Serialize report data to JSON
    let json_data = serde_json::to_string(report_data)
        .map_err(|e| InvoiceError::PdfGeneration(e.to_string()))?;

    // Write JSON to temp file
    let json_path = temp_dir.join("report_data.json");
    std::fs::write(&json_path, &json_data)?;

    // Write template with relative JSON path
    let template_content = REPORT_TEMPLATE.replace("DATA_JSON_PATH", "report_data.json");
    let template_path = temp_dir.join("report.typ");
    std::fs::write(&template_path, &template_content)?;

    // Run typst compile
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

    // Clean up temp files
    let _ = std::fs::remove_file(&template_path);
    let _ = std::fs::remove_file(&json_path);

    Ok(())
}
