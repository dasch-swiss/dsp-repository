pub fn round(val: &f64, decimals: usize) -> askama::Result<f64> {
    let multiplier = 10_f64.powi(decimals as i32);
    Ok((*val * multiplier).round() / multiplier)
}