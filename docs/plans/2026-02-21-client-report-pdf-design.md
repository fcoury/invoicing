# Client Invoice Report PDF

## Summary

New `invoice report` command that generates a professional PDF statement listing all invoices for a given client, with optional date range and payment status filters.

## CLI Interface

```
invoice report --client <id> [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--status paid|unpaid] [--open]
```

- `--client` (required): client ID from clients.toml
- `--from` / `--to` (optional): date range filter (inclusive)
- `--status` (optional): filter by payment status
- `--open` (optional): open PDF after generation

## Data Flow

1. Load config, state, and client details
2. Filter `state.history` by client ID, then optionally by date range and payment status
3. Build `ReportData` struct with company info, client info, filtered invoice rows, and summary totals
4. Serialize to JSON, write to temp file
5. New Typst template renders the PDF
6. Save to output dir as `REPORT-<client_id>-<YYYY-MM-DD>.pdf`

## ReportData Struct

```rust
struct ReportData {
    company: Company,
    client: ClientInfo,      // name, address, etc.
    invoices: Vec<ReportInvoiceRow>,  // number, date, total, status
    total: f64,
    paid: f64,
    outstanding: f64,
    currency_symbol: String,
    generated_date: String,
    filter_from: Option<String>,
    filter_to: Option<String>,
    filter_status: Option<String>,
}
```

## PDF Layout

Professional statement with company letterhead:

- Company name and address at top
- Title: "INVOICE REPORT"
- Client name and address
- Period / filter info and generation date
- Table: NUMBER, DATE, TOTAL, STATUS
- Financial summary: Total, (-) Paid, (=) Outstanding

## Components

- `ReportData` struct in `src/invoice/` with Serialize derive
- `REPORT_TEMPLATE` const in `src/pdf/typst.rs` (new Typst template)
- `generate_report_pdf()` in `src/pdf/` following existing `generate_pdf()` pattern
- `cmd_report()` handler in `src/main.rs`
- `Commands::Report` variant in clap enum

## Output

Filename: `REPORT-<client_id>-<YYYY-MM-DD>.pdf` (date is generation date)
Location: same configured output directory as invoices
