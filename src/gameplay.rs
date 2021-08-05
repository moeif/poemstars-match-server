use crate::common::Signal;
use crate::gamematch::MatchRequest;
use crate::petable::PETable;
use crate::poemtable::PoemTable;
use crate::proto;
use crate::robot::{Robot, RobotController};
use std::collections::HashMap;

const MATCH_POEM_NUM: u32 = 10;

pub struct Player {
    endpoint_id: Option<String>,
    player_id: String,
    player_name: String,
    player_level: u32,
    player_elo_score: u32,
    player_correct_rate: f64,
    game_start_timestamp: i64, // 游戏开始时间戳
    last_opt_index: i32,       // 最后一次操作的索引
    opt_bitmap: u32,           // 操作位数据, 0 正确，1 错误
    is_dirty: bool,
    robot: Option<Robot>,
    game_score: u32, // 本局游戏得分，根据操作时间来的
}

impl Player {
    fn is_all_opt_end(&self) -> bool {
        self.last_opt_index + 1 == MATCH_POEM_NUM as i32
    }

    fn on_opt(
        &mut self,
        opt: proto::CGMatchGameOpt,
        curr_timestamp: i64,
        server_index: i32,
        poem_mill_time: i64,
        poem_score: u32,
    ) {
        if opt.opt_index as i32 == server_index {
            if self.last_opt_index + 1 == opt.opt_index as i32 {
                self.last_opt_index += 1;
                self.opt_bitmap |= opt.opt_result << self.last_opt_index;

                if opt.opt_result == 0 {
                    // 在答对的情况下，计算得分
                    let server_index_start_timestamp =
                        self.game_start_timestamp + server_index as i64 * poem_mill_time;
                    let passed_time = curr_timestamp - server_index_start_timestamp;
                    let passed_percent: f64 = passed_time as f64 / poem_mill_time as f64;
                    if passed_percent < 1.0 {
                        let remaining_percent = 1.0 - passed_percent;
                        let got_score = (poem_score as f64 * remaining_percent) as u32;
                        self.game_score += got_score;
                    }
                }
            }
        }
        self.is_dirty = true;
    }

    fn update_robot_opt(
        &mut self,
        curr_timestamp: i64,
        server_index: i32,
        poem_mill_time: i64,
        poem_score: u32,
    ) {
        if self.last_opt_index != server_index {
            let server_index_start_timestamp =
                self.game_start_timestamp + server_index as i64 * poem_mill_time;
            if let Some(ref mut robot) = self.robot {
                let next_opt_time = server_index_start_timestamp + robot.next_opt_wait_time;
                if curr_timestamp > next_opt_time {
                    self.last_opt_index = server_index;
                    let opt_result = robot.get_opt_result();
                    self.opt_bitmap |= opt_result << self.last_opt_index;
                    self.is_dirty = true;
                    robot.set_next_opt_wait_time(poem_mill_time);

                    if opt_result == 0 {
                        // 在答对的情况下，计算得分
                        let passed_time = curr_timestamp - server_index_start_timestamp;
                        let passed_percent: f64 = passed_time as f64 / poem_mill_time as f64;
                        if passed_percent < 1.0 {
                            let remaining_percent = 1.0 - passed_percent;
                            let got_score = (poem_score as f64 * remaining_percent) as u32;
                            self.game_score += got_score;
                        }
                    }
                }
            }
        }
    }

    fn update_opt_timeout_status(&mut self, server_index: i32) {
        if server_index - self.last_opt_index > 1 {
            self.last_opt_index += 1;
            self.opt_bitmap |= 1 << self.last_opt_index;
            self.is_dirty = true;
        }
    }

    fn is_dirty(&mut self) -> bool {
        let tmp_is_dirty = self.is_dirty;
        self.is_dirty = false;
        return tmp_is_dirty;
    }
}

struct Game {
    id: String,           // 游戏ID
    start_timestamp: i64, // 游戏开始时间戳
    player1: Player,
    player2: Player,
    is_gaming: bool, // 游戏进行中
    is_dirty: bool,
    curr_index: i32, // 服务器根据时间判断当前处于第几个索引之间
}

impl Game {
    fn new(player1: Player, player2: Player, start_timestamp: i64) -> Self {
        Self {
            id: format!(
                "{}_{}_{}",
                player1.player_id, player2.player_id, start_timestamp
            ),
            start_timestamp,
            player1,
            player2,
            is_gaming: true,
            is_dirty: false,
            curr_index: 0,
        }
    }

    fn is_dirty(&mut self) -> bool {
        let is_dirty = self.is_dirty || self.player1.is_dirty() || self.player2.is_dirty();
        self.is_dirty = false;
        return is_dirty;
    }

    fn on_opt(
        &mut self,
        opt: proto::CGMatchGameOpt,
        curr_timestamp: i64,
        poem_mill_time: i64,
        poem_score: u32,
    ) {
        if opt.id == self.player1.player_id {
            self.player1.on_opt(
                opt,
                curr_timestamp,
                self.curr_index,
                poem_mill_time,
                poem_score,
            );
        } else if opt.id == self.player2.player_id {
            self.player2.on_opt(
                opt,
                curr_timestamp,
                self.curr_index,
                poem_mill_time,
                poem_score,
            );
        }
    }

    // 更新游戏是否结束
    fn update_end_status(&mut self) {
        if self.player1.is_all_opt_end() && self.player2.is_all_opt_end() {
            self.is_gaming = false;
            self.is_dirty = true;
        }
    }

    fn update_server_opt_index(&mut self, curr_timestamp: i64, poem_mill_time: i64) {
        self.curr_index = ((curr_timestamp - self.start_timestamp) / poem_mill_time) as i32;
    }

    fn is_game_end(&self) -> bool {
        !self.is_gaming
    }

    fn update_robot_opt(&mut self, curr_timestamp: i64, poem_mill_time: i64, poem_score: u32) {
        self.player1
            .update_robot_opt(curr_timestamp, self.curr_index, poem_mill_time, poem_score);
        self.player2
            .update_robot_opt(curr_timestamp, self.curr_index, poem_mill_time, poem_score);
    }

    fn update_opt_timeout_status(&mut self) {
        self.player1.update_opt_timeout_status(self.curr_index);
        self.player2.update_opt_timeout_status(self.curr_index);
    }

    fn gc_update_to_json(&self) -> Option<String> {
        let gc_update_game = proto::GCUpdateGame {
            game_id: self.id.clone(),
            player1_id: self.player1.player_id.clone(),
            player1_name: self.player1.player_name.clone(),
            player1_last_opt_index: self.player1.last_opt_index,
            player1_opt_bitmap: self.player1.opt_bitmap,
            player2_id: self.player2.player_id.clone(),
            player2_name: self.player2.player_name.clone(),
            player2_last_opt_index: self.player2.last_opt_index,
            player2_opt_bitmap: self.player2.opt_bitmap,
        };

        if let Ok(json_str) = serde_json::to_string(&gc_update_game) {
            if let Ok(proto_data_json_str) =
                serde_json::to_string(&proto::ProtoData::new(proto::PROTO_GCUPDATEGAME, json_str))
            {
                return Some(proto_data_json_str);
            }
        }

        return None;
    }

    fn gc_end_game_to_json(&mut self, petable: &PETable) -> Option<String> {
        if self.player1.game_score > self.player2.game_score {
            self.player1.player_level += 1;
        } else if self.player2.game_score > self.player1.game_score {
            self.player2.player_level += 1;
        }

        let (ea, eb, _) =
            petable.get_ea_eb(self.player1.player_elo_score, self.player2.player_elo_score);
        let (player1_sa, player2_sa) = if self.player1.game_score > self.player1.game_score {
            (1.0, 0.0)
        } else if self.player1.game_score < self.player2.game_score {
            (0.0, 1.0)
        } else {
            (0.5, 0.5)
        };

        let player1_new_elo_score: u32 =
            (self.player1.player_elo_score as f64 + 32.0 * (player1_sa - ea)) as u32;

        let player2_new_elo_score: u32 =
            (self.player2.player_elo_score as f64 + 32.0 * (player2_sa - eb)) as u32;

        let gc_end_game = proto::GCEndGame {
            game_id: self.id.clone(),
            player1_id: self.player1.player_id.clone(),
            player1_name: self.player1.player_name.clone(),
            player1_opt_bitmap: self.player1.opt_bitmap,
            player1_game_score: self.player1.game_score,
            player1_new_elo_score: player1_new_elo_score,
            player1_new_level: self.player1.player_level,
            player2_id: self.player2.player_id.clone(),
            player2_name: self.player2.player_name.clone(),
            player2_opt_bitmap: self.player2.opt_bitmap,
            player2_game_score: self.player2.game_score,
            player2_new_elo_score: player2_new_elo_score,
            player2_new_level: self.player2.player_level,
        };

        if let Ok(json_str) = serde_json::to_string(&gc_end_game) {
            return Some(json_str);
        }

        return None;
    }
}

pub struct MatchGameController {
    game_map: HashMap<String, Game>,
    ended_game: Vec<String>,
    poem_table: PoemTable,
    petable: PETable,
    robot_ctrl: RobotController,
    tx: std::sync::mpsc::Sender<(std::string::String, u32)>,
    poem_mill_time: i64,
    poem_score: u32,
}

impl MatchGameController {
    pub fn new(
        tx: std::sync::mpsc::Sender<(std::string::String, u32)>,
        poem_mill_time: i64,
        poem_score: u32,
    ) -> Self {
        Self {
            game_map: HashMap::new(),
            ended_game: Vec::new(),
            poem_table: PoemTable::new(),
            petable: PETable::new(),
            robot_ctrl: RobotController::new(),
            tx,
            poem_mill_time,
            poem_score,
        }
    }

    pub fn on_opt(&mut self, opt_info: proto::CGMatchGameOpt, curr_timestamp: i64) {
        if let Some(game) = self.game_map.get_mut(&opt_info.game_id) {
            game.on_opt(
                opt_info,
                curr_timestamp,
                self.poem_mill_time,
                self.poem_score,
            );
        }
    }

    pub fn update_games(&mut self, curr_timestamp: i64) -> Option<Vec<Signal>> {
        self.ended_game.clear();
        let mut some_signal_vec: Option<Vec<Signal>> = None;
        for (_, game) in self.game_map.iter_mut() {
            game.update_server_opt_index(curr_timestamp, self.poem_mill_time);
            game.update_robot_opt(curr_timestamp, self.poem_mill_time, self.poem_score);
            game.update_opt_timeout_status();
            game.update_end_status();

            if game.is_dirty() {
                if let Some(proto_json_str) = game.gc_update_to_json() {
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

                if let Some(proto_json_str) = game.gc_end_game_to_json(&self.petable) {
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

                // 将玩家的id和名字，以及分数发到另一线程，用于存到Redis里
                if let Ok(()) = self.tx.send((
                    format!("{}_{}", &game.player1.player_id, &game.player1.player_name),
                    game.player1.player_level,
                )) {
                    // data send ok
                } else {
                    // Channel send msg error
                }
                // 将第一个玩家的数据也发到另一线程，存Redis
                if let Ok(()) = self.tx.send((
                    format!("{}_{}", &game.player2.player_id, &game.player2.player_name),
                    game.player2.player_level,
                )) {
                    // data send ok
                } else {
                    // Channel send msg error
                }
            }
        }

        for game_id in self.ended_game.iter() {
            self.game_map.remove(game_id);
        }

        return some_signal_vec;
    }

    pub fn start_new_game(
        &mut self,
        player1: Player,
        player2: Player,
        curr_timestamp: i64,
    ) -> Option<Signal> {
        let player_level = player1.player_level;
        if let Some(poem_json_str) = self
            .poem_table
            .get_random_game_data(player_level, MATCH_POEM_NUM)
        {
            let player1_id = player1.player_id.clone();
            let player1_name = player1.player_name.clone();
            let player2_id = player2.player_id.clone();
            let player2_name = player2.player_name.clone();

            let gc_start_game = proto::GCStartGame {
                player1_id: player1_id,
                player1_name: player1_name,
                player2_id: player2_id,
                player2_name: player2_name,
                poem_data_str: poem_json_str,
            };

            // 创建消息同步 Signal
            if let Ok(gc_start_game_json_str) = serde_json::to_string(&gc_start_game) {
                let signal = Signal::Sync(
                    player1.endpoint_id.clone(),
                    player2.endpoint_id.clone(),
                    gc_start_game_json_str,
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

    pub fn create_robot_player(
        &mut self,
        competitor_player: &Player,
        curr_timestamp: i64,
        poem_mill_time: i64,
    ) -> Player {
        let robot = self.robot_ctrl.get_robot(
            competitor_player.player_level,
            competitor_player.player_elo_score,
            competitor_player.player_correct_rate,
            poem_mill_time,
        );
        Player {
            endpoint_id: None,
            player_id: robot.id.clone(),
            player_name: robot.name.clone(),
            player_level: robot.level,
            player_elo_score: robot.elo_score,
            player_correct_rate: robot.correct_rate,
            game_start_timestamp: curr_timestamp,
            last_opt_index: -1,
            opt_bitmap: 0,
            is_dirty: false,
            robot: Some(robot),
            game_score: 0,
        }
    }
}

pub fn create_player_from_match(match_reqeust: MatchRequest, curr_timestamp: i64) -> Player {
    Player {
        endpoint_id: match_reqeust.endpoint_id,
        player_id: match_reqeust.player_id,
        player_name: match_reqeust.player_name,
        player_level: match_reqeust.player_level,
        player_elo_score: match_reqeust.player_elo_score,
        player_correct_rate: match_reqeust.player_correct_rate,
        game_start_timestamp: curr_timestamp,
        last_opt_index: -1,
        opt_bitmap: 0,
        is_dirty: false,
        robot: None,
        game_score: 0,
    }
}
