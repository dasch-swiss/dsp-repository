
use askama::Template;
use crate::views::html::home::IndexTemplate;
pub fn get_home_page() -> String {
    let view = IndexTemplate {};
    view.render().unwrap()
}