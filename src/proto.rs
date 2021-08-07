use serde::{Deserialize, Serialize};
use std::str;

pub const PROTO_CGSTARTMATCH: u64 = 1001;
pub const PROTO_GCSTARTMATCH: u64 = 2001;
pub const PROTO_CGMATCHGAMEOPT: u64 = 1002;
pub const PROTO_GCSTARTGAME: u64 = 2002;
pub const PROTO_GCUPDATEGAME: u64 = 2003;
pub const PROTO_GCENDGAME: u64 = 2004;

pub trait GCProtoBase64 {
    fn to_base64_json_str(&self) -> Option<String>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CGStartMatch {
    pub id: String,        // 玩家ID
    pub name: String,      // 玩家昵称
    pub level: u32,        // 胜利次数
    pub elo_score: u32,    // elo 分值
    pub correct_rate: f64, // 正确率
}

// Debug Code
impl GCProtoBase64 for CGStartMatch {
    fn to_base64_json_str(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            log::info!("CGStartMatch: {:?}", json_str);
            let base64_json_str = base64::encode(json_str);
            return Some(base64_json_str);
        }
        return None;
    }
}

#[derive(Serialize)]
pub struct GCStartMatch {
    pub code: i32,
}

impl GCProtoBase64 for GCStartMatch {
    fn to_base64_json_str(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            log::info!("GCStartMatch: {:?}", json_str);
            let base64_json_str = base64::encode(json_str);
            return Some(base64_json_str);
        }
        return None;
    }
}

#[derive(Serialize)]
pub struct GCStartGame {
    pub game_id: String,
    pub player1_id: String,
    pub player1_name: String,
    pub player2_id: String,
    pub player2_name: String,
    pub poem_data_str: String,
}

impl GCProtoBase64 for GCStartGame {
    fn to_base64_json_str(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            log::info!("GCStartGame: {:?}", json_str);
            let base64_json_str = base64::encode(json_str);
            return Some(base64_json_str);
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
    pub player1_next_opt_index: i32,
    pub player1_opt_bitmap: u32,
    pub player2_id: String,
    pub player2_name: String,
    pub player2_next_opt_index: i32,
    pub player2_opt_bitmap: u32,
}

impl GCProtoBase64 for GCUpdateGame {
    fn to_base64_json_str(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            log::info!("GCUpdateGame: {:?}", json_str);
            let base64_json_str = base64::encode(json_str);
            return Some(base64_json_str);
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

impl GCProtoBase64 for GCEndGame {
    fn to_base64_json_str(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            log::info!("GCEndGame: {:?}", json_str);
            let base64_json_str = base64::encode(json_str);
            return Some(base64_json_str);
        }
        return None;
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProtoData {
    pub proto_id: u64,
    pub proto_json_str: String,
}

impl ProtoData {
    pub fn gc_to_json_string(proto_id: u64, proto: impl GCProtoBase64) -> Option<String> {
        if let Some(proto_json_str) = proto.to_base64_json_str() {
            let proto_data = Self {
                proto_id: proto_id,
                proto_json_str: proto_json_str,
            };

            if let Ok(proto_json_str) = serde_json::to_string(&proto_data) {
                return Some(proto_json_str);
            }
        }

        return None;
    }

    // 收到客户端发来的数据，解析出 protoId和具体协议的 base64_json_str
    pub fn cg_to_proto_json_str(json_str: String) -> Option<(u64, String)> {
        if let Ok(proto_data) = serde_json::from_str::<ProtoData>(&json_str) {
            return Some((proto_data.proto_id, proto_data.proto_json_str));
        }
        return None;
    }

    // 将 Base64 json str 转成具体的协议
    pub fn deserialize_proto<T>(base64_json_str: String) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Ok(bytes) = base64::decode(base64_json_str) {
            if let Ok(raw_json_str) = str::from_utf8(&bytes) {
                log::info!("Client -> Server JsonStr: {:?}", raw_json_str);
                if let Ok(proto) = serde_json::from_str::<T>(raw_json_str) {
                    return Some(proto);
                } else {
                    log::info!("Error!, Deserialize json to CG proto failed!");
                }
            }
        }
        return None;
    }
}
