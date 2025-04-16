mod filters;

use types::calculator::CashFlowRow;

#[derive(askama::Template)]
#[template(path = "html/calculator/index.html")]
pub(crate) struct CalculatorTemplate;

#[derive(askama::Template)]
#[template(path = "html/calculator/table.html")]
pub struct DcfResultTableTemplate {
    pub rows: Vec<CashFlowRow>,
    pub total_intrinsic_value: f64, // sum of discounted values
}


