#[derive(Clone)]
pub enum Signal {
    Send(String, String),
    Sync(String, String, String),
}
