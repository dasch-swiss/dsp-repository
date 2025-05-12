use askama::Template;
use types::calculator::DcfResult;

pub fn get_index_page() -> String {
    let page = crate::views::html::calculator::CalculatorTemplate {};
    page.render().unwrap()
}

pub fn get_style_css() -> &'static str {
    include_str!("../../templates/html/calculator/style.css")
}

pub fn get_result_table_page_fragment(context: &DcfResult) -> String {
    let view = crate::views::html::calculator::DcfResultTableTemplate {
        rows: context.rows.clone(),
        total_intrinsic_value: context.total_intrinsic_value,
    };
    view.render().unwrap()
}
