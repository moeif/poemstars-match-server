#[derive(Clone)]
pub enum Signal {
    Send(String, String),
    Sync(Option<String>, Option<String>, String),
}
