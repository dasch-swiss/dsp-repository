use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DcfForm {
    pub fcf: f64,
    pub growth: f64,
    pub discount: f64,
    pub terminal: f64,
    pub years: u32,
}

#[derive(Clone, Debug)]
pub struct CashFlowRow {
    pub year: String,    // "1", "2", ..., "Terminal"
    pub fcf: f64,        // in millions
    pub discounted: f64, // in millions
}

#[derive(Clone, Debug)]
pub struct DcfResult {
    pub rows: Vec<CashFlowRow>,
    pub total_intrinsic_value: f64,
}

