use types::calculator::{CashFlowRow, DcfResult};

#[derive(Debug, Clone)]
pub struct CalculatorServiceImpl;

impl CalculatorServiceImpl {
    pub fn compute_dcf_result(
        &self,
        fcf: f64,
        growth: f64,
        discount: f64,
        terminal: f64,
        years: u32,
    ) -> DcfResult {
        let mut rows = Vec::new();
        let mut total = 0.0;

        for year in 1..=years {
            let projected_fcf = fcf * (1.0 + growth).powi(year as i32);
            let discounted = projected_fcf / (1.0 + discount).powi(year as i32);

            rows.push(CashFlowRow {
                year: year.to_string(),
                fcf: projected_fcf,
                discounted,
            });

            total += discounted;
        }

        // Terminal calculation
        let last_fcf = fcf * (1.0 + growth).powi(years as i32);
        let terminal_value = last_fcf * (1.0 + terminal) / (discount - terminal);
        let discounted_terminal = terminal_value / (1.0 + discount).powi(years as i32);

        rows.push(CashFlowRow {
            year: "Terminal".to_string(),
            fcf: terminal_value,
            discounted: discounted_terminal,
        });

        total += discounted_terminal;

        DcfResult {
            rows,
            total_intrinsic_value: total,
        }
    } 
}
