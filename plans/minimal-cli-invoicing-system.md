# Minimal CLI Invoicing System

## Overview

Build a minimal command-line invoicing tool that generates professional PDF invoices from TOML configuration files. Users configure their company, clients, and line items once, then generate invoices by specifying client, items, and quantities.

**Target User:** Freelancers and small businesses who want a simple, scriptable invoice generation without SaaS subscriptions.

## Problem Statement

Creating invoices manually is tedious. Existing solutions are either:
- Overkill SaaS products with monthly fees
- Spreadsheet templates requiring manual PDF export
- Complex accounting software

A minimal CLI tool that reads TOML configs and outputs PDFs fills the gap for developers and power users who prefer terminal workflows.

## Proposed Solution

A single-binary CLI tool (Rust with Typst for PDF generation) that:
1. Reads company/client/item data from TOML files in `~/.invoice/`
2. Generates professionally formatted PDF invoices
3. Tracks invoice numbering automatically
4. Provides simple commands for listing clients and items

### Core Commands

```bash
# Generate an invoice
invoice generate --client acme --item consulting:8 --item development:40

# List configured clients
invoice clients

# List available line items
invoice items

# Show next invoice number and config status
invoice status

# Initialize config directory with templates
invoice init
```

## Technical Approach

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Layer                             │
│  (clap for argument parsing, subcommands)                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Config Layer                            │
│  (TOML parsing with serde, validation)                      │
│  - config.toml (company settings)                           │
│  - clients.toml (client definitions)                        │
│  - items.toml (line item catalog)                           │
│  - state.toml (invoice counter)                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Invoice Engine                           │
│  - Calculate line totals                                    │
│  - Apply tax rates                                          │
│  - Generate invoice number                                  │
│  - Format currency                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      PDF Generator                           │
│  (Typst CLI for PDF generation from template)               │
│  - invoice.typ template                                     │
│  - JSON data injection                                      │
└─────────────────────────────────────────────────────────────┘
```

### File Structure

```
invoice-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── lib.rs               # Public API for testing
│   ├── config/
│   │   ├── mod.rs           # Config module
│   │   ├── company.rs       # Company settings
│   │   ├── client.rs        # Client definitions
│   │   └── item.rs          # Line item definitions
│   ├── invoice/
│   │   ├── mod.rs
│   │   ├── generator.rs     # Invoice calculation
│   │   └── numbering.rs     # Invoice number formatting
│   ├── pdf/
│   │   ├── mod.rs
│   │   └── typst.rs         # Typst PDF generation
│   └── error.rs             # Custom error types
├── templates/
│   └── invoice.typ          # Default Typst template
└── tests/
    ├── cli_tests.rs
    └── invoice_tests.rs
```

### Config Directory Structure

```
~/.invoice/
├── config.toml              # Company settings, invoice format
├── clients.toml             # Client definitions
├── items.toml               # Line item catalog
├── state.toml               # Invoice counter
├── templates/
│   └── invoice.typ          # Custom template (optional)
└── output/                  # Generated invoices
    ├── INV-2026-0001.pdf
    └── INV-2026-0002.pdf
```

### TOML Schema Definitions

#### config.toml

```toml
[company]
name = "Your Company LLC"
address = "123 Business Street"
city = "San Francisco"
state = "CA"
zip = "94102"
country = "USA"
email = "billing@yourcompany.com"
phone = "+1-555-123-4567"           # optional
tax_id = "12-3456789"               # optional

[invoice]
number_format = "INV-{year}-{seq:04}"  # e.g., INV-2026-0001
currency = "USD"
currency_symbol = "$"
due_days = 30
tax_rate = 0.0                      # 0.0825 for 8.25%

[pdf]
output_dir = "~/.invoice/output"    # where PDFs are saved
# logo_path = "~/.invoice/logo.png" # optional, future feature
```

#### clients.toml

```toml
[acme]
name = "Acme Corporation"
contact = "Jane Smith"              # optional
email = "jane@acme.com"
address = "456 Corporate Ave"
city = "Los Angeles"
state = "CA"
zip = "90001"
country = "USA"                     # optional, defaults to company country

[widget-inc]
name = "Widget Industries"
email = "bob@widgets.io"
address = "789 Industrial Blvd"
city = "Chicago"
state = "IL"
zip = "60601"
```

#### items.toml

```toml
[consulting]
description = "Technical Consulting"
rate = 150.00
unit = "hour"

[development]
description = "Software Development"
rate = 125.00
unit = "hour"

[project-setup]
description = "Project Setup & Configuration"
rate = 500.00
unit = "flat"                       # fixed price, quantity = 1
```

#### state.toml

```toml
[counter]
last_number = 0
last_year = 2026

[[history]]
number = "INV-2026-0001"
client = "acme"
date = 2026-01-15
total = 1875.00
file = "INV-2026-0001.pdf"
```

### Implementation Phases

#### Phase 1: Foundation

- [ ] Set up Rust project with Cargo
- [ ] Implement CLI argument parsing with clap
- [ ] Create config loading and validation
- [ ] Implement `invoice init` command
- [ ] Create error types and handling

**Deliverables:**
- `src/main.rs` - CLI entry point
- `src/config/*.rs` - TOML parsing
- `src/error.rs` - Error types
- Working `init` command that creates config templates

#### Phase 2: Core Invoice Logic

- [ ] Implement invoice calculation engine
- [ ] Build invoice number generator
- [ ] Create `clients` and `items` list commands
- [ ] Implement `status` command
- [ ] Add state.toml management

**Deliverables:**
- `src/invoice/*.rs` - Invoice logic
- Working `clients`, `items`, `status` commands
- Automatic invoice number incrementing

#### Phase 3: PDF Generation

- [ ] Create Typst invoice template
- [ ] Implement PDF generation via Typst CLI
- [ ] Add `generate` command
- [ ] Save invoices and update history

**Deliverables:**
- `templates/invoice.typ` - Typst template
- `src/pdf/*.rs` - PDF generation
- Working `generate` command producing PDFs

#### Phase 4: Polish & Testing

- [ ] Add comprehensive error messages
- [ ] Write integration tests
- [ ] Add `--help` documentation
- [ ] Handle edge cases (year rollover, missing configs)
- [ ] Add `--output` flag for custom PDF path

**Deliverables:**
- Test suite in `tests/`
- Polished CLI help text
- Robust error handling

## Acceptance Criteria

### Functional Requirements

- [ ] `invoice init` creates `~/.invoice/` with template TOML files
- [ ] `invoice clients` lists all configured clients
- [ ] `invoice items` lists all available line items with rates
- [ ] `invoice status` shows next invoice number and config status
- [ ] `invoice generate -c <client> -i <item>:<qty>` generates PDF invoice
- [ ] Invoice numbers auto-increment and persist across sessions
- [ ] Generated PDFs include: company info, client info, line items, totals
- [ ] Tax calculation applied when configured

### Non-Functional Requirements

- [ ] Single binary with no runtime dependencies (except Typst CLI)
- [ ] Sub-second PDF generation
- [ ] Clear error messages for common mistakes
- [ ] UTF-8 support for international characters

### Quality Gates

- [ ] All commands have `--help` documentation
- [ ] Integration tests for each command
- [ ] Unit tests for invoice calculations
- [ ] Handles missing/malformed TOML gracefully

## MVP Scope

### Included in MVP

- `init`, `generate`, `clients`, `items`, `status` commands
- TOML-based configuration
- PDF generation via Typst
- Automatic invoice numbering
- Basic tax calculation (single rate)

### Deferred to Future

- Interactive client/item management (`clients add`, `items add`)
- Multiple tax rates per client/item
- Logo on invoice
- Invoice search/history query
- Multiple currencies
- Invoice regeneration
- Dry-run/preview mode
- JSON output format for scripting

## Dependencies

### Required

- **Rust** - Core language
- **clap** - CLI argument parsing
- **serde** + **toml** - TOML parsing
- **chrono** - Date handling
- **directories** - XDG config path resolution
- **Typst CLI** - PDF generation (external dependency)

### Development

- **assert_cmd** - CLI testing
- **predicates** - Test assertions
- **tempfile** - Test fixtures

## Risk Analysis

| Risk | Mitigation |
|------|------------|
| Typst CLI not installed | Detect and show clear install instructions |
| State.toml corruption | Validate on load, error with recovery steps |
| Concurrent generation | Use file locking or detect and error |
| Year rollover mid-session | Check year on each generation, reset if needed |

## Success Metrics

- Generate invoice in < 1 second
- Zero data loss from counter management
- Users can go from install to first invoice in < 5 minutes

## Example Workflow

```bash
# Install (future: via cargo install or release binary)
cargo install invoice-cli

# Initialize configuration
invoice init

# Edit your company details
$EDITOR ~/.invoice/config.toml

# Add a client
$EDITOR ~/.invoice/clients.toml
# Add: [acme]
#      name = "Acme Corp"
#      email = "billing@acme.com"
#      ...

# Add line items
$EDITOR ~/.invoice/items.toml
# Add: [consulting]
#      description = "Technical Consulting"
#      rate = 150.00
#      unit = "hour"

# Check setup
invoice status
# Output: Next invoice: INV-2026-0001
#         Clients: 1  Items: 1

# Generate invoice
invoice generate --client acme --item consulting:8

# Output: Generated INV-2026-0001.pdf
#         Client: Acme Corp
#         Total: $1,200.00
#         Saved to: ~/.invoice/output/INV-2026-0001.pdf
```

## References

### Internal

- This is a greenfield project, no existing code.

### External

- [TOML v1.0.0 Specification](https://toml.io/en/v1.0.0)
- [Clap Documentation](https://docs.rs/clap/latest/clap/)
- [Typst Automated PDF Generation](https://typst.app/blog/2025/automated-generation/)
- [Invoice Numbering Best Practices](https://www.invoicesimple.com/blog/invoice-number)
- [Cargo Project Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)

### Related Tools

- [invoist - Typst Invoice Template](https://github.com/WilstonOreo/invoist)
- [invoice-pro Typst Package](https://typst.app/universe/package/invoice-pro/)
