mod filters;

use askama::Template;
use serde::Deserialize;

#[derive(askama::Template)]
#[template(path = "calculator/index.html")]
struct CalculatorTemplate;

pub fn index() -> String {
    let page = CalculatorTemplate {};
    page.render().unwrap()
}

pub fn style() -> &'static str {
    include_str!("../../templates/calculator/style.css")
}

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
    pub year: String,         // "1", "2", ..., "Terminal"
    pub fcf: f64,             // in millions
    pub discounted: f64,      // in millions
}

#[derive(askama::Template)]
#[template(path = "calculator/table.html")]
pub struct DcfTableContext {
    pub rows: Vec<CashFlowRow>,
    pub total_intrinsic_value: f64,  // sum of discounted values
}

pub fn result_table(context: &DcfTableContext) -> String {
    let table = DcfTableContext {
        rows: context.rows.clone(),
        total_intrinsic_value: context.total_intrinsic_value
    };
    table.render().unwrap()
}