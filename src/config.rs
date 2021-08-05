use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub area: String,
    pub port: u32,
    pub poem_mill_time: i64,
    pub poem_score: u32,
    pub match_data_key_name: String,
}

impl ServerConfig {
    pub fn new() -> Self {
        let mut file = File::open("./configs/server_config.json").unwrap();
        let mut config_content = String::new();
        file.read_to_string(&mut config_content).unwrap();
        let config = serde_json::from_str::<Self>(&config_content).unwrap();
        return config;
    }
}
