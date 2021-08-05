use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTO_CGSTARTMATCH: u64 = 1001;
pub const PROTO_GCSTARTMATCH: u64 = 2001;
pub const PROTO_CGMATCHGAMEOPT: u64 = 1002;
pub const PROTO_GCSTARTGAME: u64 = 2002;
pub const PROTO_GCUPDATEGAME: u64 = 2003;
pub const PROTO_GCENDGAME: u64 = 2004;

#[derive(Serialize, Deserialize, Debug)]
pub struct CGStartMatch {
    pub id: String,        // 玩家ID
    pub name: String,      // 玩家昵称
    pub level: u32,        // 胜利次数
    pub elo_score: u32,    // elo 分值
    pub correct_rate: f64, // 正确率
}

#[derive(Serialize)]
pub struct GCStartMatch {
    pub code: i32,
}

impl GCStartMatch {
    pub fn gc_to_json(code: i32) -> Option<String> {
        let proto = GCStartMatch { code };

        if let Ok(json_str) = serde_json::to_string(&proto) {
            return ProtoData::new(PROTO_GCSTARTMATCH, json_str);
        }

        return None;
    }
}

#[derive(Serialize)]
pub struct GCStartGame {
    pub player1_id: String,
    pub player1_name: String,
    pub player2_id: String,
    pub player2_name: String,
    pub poem_data_str: String,
}

impl GCStartGame {
    pub fn gc_to_json(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            return ProtoData::new(PROTO_GCSTARTGAME, json_str);
        }
        return None;
    }
}

#[derive(Deserialize, Debug)]
pub struct CGMatchGameOpt {
    pub id: String,      // 玩家ID
    pub game_id: String, // 游戏ID
    pub opt_index: u32,  // 操作了哪个索引
    pub opt_result: u32, // 操作的结果，0对，1错
}

#[derive(Serialize)]
pub struct GCUpdateGame {
    pub game_id: String,
    pub player1_id: String,
    pub player1_name: String,
    pub player1_last_opt_index: i32,
    pub player1_opt_bitmap: u32,
    pub player2_id: String,
    pub player2_name: String,
    pub player2_last_opt_index: i32,
    pub player2_opt_bitmap: u32,
}

impl GCUpdateGame {
    pub fn gc_to_json(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            return ProtoData::new(PROTO_GCUPDATEGAME, json_str);
        }
        return None;
    }
}

#[derive(Serialize)]
pub struct GCEndGame {
    pub game_id: String,
    pub player1_id: String,
    pub player1_name: String,
    pub player1_opt_bitmap: u32,
    pub player1_game_score: u32,
    pub player1_new_elo_score: u32,
    pub player1_new_level: u32,
    pub player2_id: String,
    pub player2_name: String,
    pub player2_opt_bitmap: u32,
    pub player2_game_score: u32,
    pub player2_new_elo_score: u32,
    pub player2_new_level: u32,
}

impl GCEndGame {
    pub fn gc_to_json(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            return ProtoData::new(PROTO_GCENDGAME, json_str);
        }
        return None;
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProtoData {
    pub proto_id: u64,
    pub proto_json_str: Value,
}

impl ProtoData {
    pub fn new(proto_id: u64, json_str: String) -> Option<String> {
        if let Ok(json_value) = serde_json::from_str::<Value>(&json_str) {
            let proto_data = Self {
                proto_id: proto_id,
                proto_json_str: json_value,
            };

            if let Ok(proto_json_str) = serde_json::to_string(&proto_data) {
                return Some(proto_json_str);
            }
        }

        return None;
    }
}
