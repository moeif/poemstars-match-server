use serde::{Deserialize, Serialize};

pub const PROTO_CGSTARTMATCH: u64 = 1001;
pub const PROTO_GCSTARTMATCH: u64 = 2001;
pub const PROTO_CGMATCHGAMEOPT: u64 = 1002;
pub const PROTO_UPDATEGAME: u64 = 1003;

#[derive(Deserialize)]
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
            let proto_data = ProtoData::new(PROTO_GCSTARTMATCH, json_str);
            if let Ok(proto_data_json_str) = serde_json::to_string(&proto_data) {
                return Some(proto_data_json_str);
            }
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

#[derive(Deserialize)]
pub struct CGMatchGameOpt {
    pub id: String,      // 玩家ID
    pub game_id: String, // 游戏ID
    pub opt_index: u32,  // 操作了哪个索引
    pub opt_result: u32, // 操作的结果，0对，1错
}

#[derive(Serialize, Deserialize)]
pub struct ProtoData {
    pub proto_id: u64,
    pub proto_json_str: String,
}

impl ProtoData {
    pub fn new(proto_id: u64, json_str: String) -> Self {
        Self {
            proto_id,
            proto_json_str: json_str,
        }
    }
}
