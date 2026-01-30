use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub company: Company,
    pub invoice: InvoiceSettings,
    pub pdf: PdfSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Company {
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub tax_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceSettings {
    pub number_format: String,
    pub currency: String,
    pub currency_symbol: String,
    pub due_days: u32,
    #[serde(default)]
    pub tax_rate: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PdfSettings {
    pub output_dir: String,
}
