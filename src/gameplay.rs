use crate::common::Signal;
use crate::gamematch;
use crate::poemtable::PoemTable;
use crate::proto;
use serde::Serialize;
use std::collections::HashMap;

const MATCH_POEM_NUM: u32 = 10;
const MATCH_TIME: u8 = 5;
const OPT_TIMEOUT: i64 = 10;

#[derive(Serialize)]
struct Player {
    pub id: String,
    pub endpoint_id: Option<String>,
    pub name: String,
    pub last_opt_timestamp: i64, // 最后一次的操作时间
    pub last_opt_index: i32,     // 最后一次操作的索引
    pub opt_bitmap: u32,         // 操作位数据, 0 正确，1 错误
    pub is_dirty: bool,
}

impl Player {
    fn new(endpoint_id: Option<String>, id: String, name: String, last_opt_timestamp: i64) -> Self {
        Self {
            id,
            endpoint_id,
            name,
            last_opt_timestamp,
            last_opt_index: -1,
            opt_bitmap: 0,
            is_dirty: false,
        }
    }

    fn is_all_opt_end(&self) -> bool {
        self.last_opt_index + 1 == MATCH_POEM_NUM as i32
    }

    fn on_opt(&mut self, opt: proto::CGMatchGameOpt, curr_timestamp: i64) {
        self.last_opt_timestamp = curr_timestamp;
        if self.last_opt_index + 1 == opt.opt_index as i32 {
            self.last_opt_index += 1;
            self.opt_bitmap |= opt.opt_result << self.last_opt_index;
        }
        self.is_dirty = true;
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64) {
        if curr_timestamp - self.last_opt_timestamp > OPT_TIMEOUT {
            if self.last_opt_index + 1 < MATCH_POEM_NUM as i32 {
                self.last_opt_index += 1;
                self.opt_bitmap |= 1 << self.last_opt_index;
                self.is_dirty = true;
            }
        }
    }

    fn is_dirty(&mut self) -> bool {
        let tmp_is_dirty = self.is_dirty;
        self.is_dirty = false;
        return tmp_is_dirty;
    }
}

#[derive(Serialize)]
struct Game {
    pub id: String,           // 游戏ID
    pub start_timestamp: i64, // 游戏开始时间戳
    pub player1: Player,
    pub player2: Player,
    pub is_gaming: bool, // 游戏进行中
    pub is_dirty: bool,
}

impl Game {
    fn new(player1: Player, player2: Player, start_timestamp: i64) -> Self {
        Self {
            id: format!("{}_{}_{}", player1.id, player2.id, start_timestamp),
            start_timestamp,
            player1,
            player2,
            is_gaming: true,
            is_dirty: false,
        }
    }

    fn is_dirty(&mut self) -> bool {
        let is_dirty = self.is_dirty || self.player1.is_dirty() || self.player2.is_dirty();
        self.is_dirty = false;
        return is_dirty;
    }

    fn on_opt(&mut self, opt: proto::CGMatchGameOpt, curr_timestamp: i64) {
        if opt.id == self.player1.id {
            self.player1.on_opt(opt, curr_timestamp);
        } else if opt.id == self.player2.id {
            self.player2.on_opt(opt, curr_timestamp);
        }
    }

    // 更新游戏是否结束
    fn update_end_status(&mut self) {
        if self.player1.is_all_opt_end() && self.player2.is_all_opt_end() {
            self.is_gaming = false;
            self.is_dirty = true;
        }
    }

    fn is_game_end(&self) -> bool {
        !self.is_gaming
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64) {
        self.player1.update_opt_timeout_status(curr_timestamp);
        self.player2.update_opt_timeout_status(curr_timestamp);
    }

    fn gc_to_json(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            if let Ok(proto_data_json_str) =
                serde_json::to_string(&proto::ProtoData::new(proto::PROTO_UPDATEGAME, json_str))
            {
                return Some(proto_data_json_str);
            }
        }
        return None;
    }
}

pub struct MatchGameController {
    game_map: HashMap<String, Game>,
    last_update_timestamp: i64,
    ended_game: Vec<String>,
    poem_table: PoemTable,
}

impl MatchGameController {
    pub fn new() -> Self {
        Self {
            game_map: HashMap::new(),
            last_update_timestamp: -1,
            ended_game: Vec::new(),
            poem_table: PoemTable::new(),
        }
    }

    pub fn on_opt(&mut self, opt_info: proto::CGMatchGameOpt, curr_timestamp: i64) {
        if let Some(game) = self.game_map.get_mut(&opt_info.game_id) {
            game.on_opt(opt_info, curr_timestamp);
        }
    }

    pub fn update_games(&mut self, curr_timestamp: i64) -> Option<Vec<Signal>> {
        println!("更新游戏: {}", curr_timestamp);
        self.ended_game.clear();
        let mut some_signal_vec: Option<Vec<Signal>> = None;
        for (_, game) in self.game_map.iter_mut() {
            game.update_opt_timeout_status(curr_timestamp);
            game.update_end_status();

            if game.is_dirty() {
                if let Some(proto_json_str) = game.gc_to_json() {
                    let signal = Signal::Sync(
                        game.player1.endpoint_id.clone(),
                        game.player2.endpoint_id.clone(),
                        proto_json_str,
                    );

                    if let Some(ref mut signal_vec) = some_signal_vec {
                        signal_vec.push(signal);
                    } else {
                        let mut vec: Vec<Signal> = Vec::new();
                        vec.push(signal);
                        some_signal_vec = Some(vec);
                    }
                }
            }

            if game.is_game_end() {
                self.ended_game.push(game.id.clone());
            }
        }

        for game_id in self.ended_game.iter() {
            self.game_map.remove(game_id);
        }

        return some_signal_vec;
    }

    pub fn create_new_game(
        &mut self,
        match_result: gamematch::MatchResult,
        curr_timestamp: i64,
    ) -> Option<Signal> {
        let main_player_level = match_result.match_req1.cg_match_info.level;
        if let Some(poem_json_str) = self
            .poem_table
            .get_random_game_data(main_player_level, MATCH_POEM_NUM)
        {
            let player1_id = match_result.match_req1.cg_match_info.id.clone();
            let player1_name = match_result.match_req1.cg_match_info.name.clone();
            let (player2_id, player2_name) = match match_result.get_match_req2_id_name() {
                Some((id, name)) => (id, name),
                None => match &match_result.robot {
                    Some(ref robot) => (robot.id.clone(), robot.name.clone()),
                    None => (String::from("Unknow"), String::from("Unknow")),
                },
            };

            let gc_start_game = proto::GCStartGame {
                player1_id: player1_id.clone(),
                player1_name: player1_name.clone(),
                player2_id: player2_id.clone(),
                player2_name: player2_name.clone(),
                poem_data_str: poem_json_str,
            };

            // 创建消息同步 Signal
            if let Ok(gc_start_game_json_str) = serde_json::to_string(&gc_start_game) {
                let player1_endpoint_id = match_result.get_match_req1_endpoint_id();
                let player2_endpoint_id: Option<String> = match_result.get_match_req2_endpoint_id();

                let signal = Signal::Sync(
                    player1_endpoint_id.clone(),
                    player2_endpoint_id.clone(),
                    gc_start_game_json_str,
                );

                // 创建游戏
                let player1 = Player::new(
                    player1_endpoint_id,
                    player1_id,
                    player1_name,
                    curr_timestamp,
                );

                let player2 = Player::new(
                    player2_endpoint_id,
                    player2_id,
                    player2_name,
                    curr_timestamp,
                );

                let game = Game::new(player1, player2, curr_timestamp);
                self.game_map.insert(game.id.clone(), game);
                return Some(signal);
            } else {
                println!("创建GCStartGame消息时Json序列化失败，无法进行游戏!");
            }
        } else {
            println!("诗词数据生成失败，无法进行游戏!");
        }

        return None;
    }
}
