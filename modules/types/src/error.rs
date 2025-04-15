
#[derive(Debug)]
pub enum  AppError {
    Message(String),
    Msg(&'static str)
}
