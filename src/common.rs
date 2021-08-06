#[derive(Clone)]
pub enum Signal {
    Send(String, String),
    Sync(Option<String>, Option<String>, String),
}

#[derive(Clone)]
pub enum RedisOpt {
    GamePlayerData(String, u32),
    GameStatus(u32),
    ServerStatus(u32),
}
