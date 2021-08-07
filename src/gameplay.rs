use crate::common::{RedisOpt, Signal};
use crate::gamematch::MatchRequest;
use crate::petable::PETable;
use crate::poemtable::PoemTable;
use crate::proto;
use crate::robot::{Robot, RobotController};
use std::collections::HashMap;

const MATCH_POEM_NUM: u32 = 10;
const POEM_RESULT_WAIT: i64 = 2500; //ms, 比客户端多1s

#[derive(Debug)]
pub struct Player {
    endpoint_id: Option<String>,
    player_id: String,
    player_name: String,
    player_level: u32,
    player_elo_score: u32,
    player_correct_rate: f64,
    game_start_timestamp: i64, // 游戏开始时间戳
    next_opt_index: i32,
    next_opt_timeout_timestamp: i64,
    opt_bitmap: u32, // 操作位数据, 0 正确，1 错误
    is_dirty: bool,
    robot: Option<Robot>,
    game_score: u32, // 本局游戏得分，根据操作时间来的
}

impl Player {
    fn is_all_opt_end(&self) -> bool {
        self.next_opt_index >= MATCH_POEM_NUM as i32
    }

    fn on_opt(
        &mut self,
        opt: proto::CGMatchGameOpt,
        curr_timestamp: i64,
        poem_mill_time: i64,
        poem_score: u32,
    ) {
        if self.next_opt_index == opt.opt_index as i32 {
            self.opt_bitmap |= opt.opt_result << self.next_opt_index;
            log::info!("On Player {} OPT", self.player_name);
            if opt.opt_result == 0 {
                // 在答对的情况下，计算得分
                let remaining_time = self.next_opt_timeout_timestamp - curr_timestamp;
                if remaining_time < 0 || remaining_time > poem_mill_time {
                    log::error!("逻辑错误，剩余时间不在合理范围内, {} ", remaining_time);
                } else {
                    let remaining_percent = remaining_time as f64 / poem_mill_time as f64;
                    let got_score = (poem_score as f64 * remaining_percent) as u32;
                    self.game_score += got_score;
                }
            }
            self.next_opt_index += 1;
            self.next_opt_timeout_timestamp = curr_timestamp + poem_mill_time + POEM_RESULT_WAIT;
            self.is_dirty = true;
        } else {
            log::error!("OPT failed with index!");
        }
    }

    fn update_robot_opt(&mut self, curr_timestamp: i64, poem_mill_time: i64, poem_score: u32) {
        if let Some(ref mut robot) = self.robot {
            let next_opt_time = self.next_opt_timeout_timestamp - robot.next_early_opt_time;
            if curr_timestamp >= next_opt_time {
                // 执行机器人操作
                let opt_result = robot.get_opt_result();
                self.opt_bitmap |= opt_result << self.next_opt_index;
                self.is_dirty = true;
                robot.set_next_opt_wait_time(poem_mill_time);
                log::info!("ROBOT {} auto OPT!", self.player_name);

                let remaining_percent = robot.next_early_opt_time as f64 / poem_mill_time as f64;
                let got_score = (poem_score as f64 * remaining_percent) as u32;
                self.game_score += got_score;

                self.next_opt_index += 1;
                self.next_opt_timeout_timestamp =
                    curr_timestamp + poem_mill_time + POEM_RESULT_WAIT;
            }
        }
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64, poem_mill_time: i64) {
        if curr_timestamp > self.next_opt_timeout_timestamp {
            self.opt_bitmap |= 1 << self.next_opt_index;
            self.next_opt_index += 1;
            self.next_opt_timeout_timestamp = curr_timestamp + poem_mill_time + POEM_RESULT_WAIT;
            self.is_dirty = true;
            log::info!("Player {} OPT Timeout, Auto Failed!", self.player_name);
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
            self.player1
                .on_opt(opt, curr_timestamp, poem_mill_time, poem_score);
        } else if opt.id == self.player2.player_id {
            self.player2
                .on_opt(opt, curr_timestamp, poem_mill_time, poem_score);
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

    fn update_robot_opt(&mut self, curr_timestamp: i64, poem_mill_time: i64, poem_score: u32) {
        self.player1
            .update_robot_opt(curr_timestamp, poem_mill_time, poem_score);
        self.player2
            .update_robot_opt(curr_timestamp, poem_mill_time, poem_score);
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64, poem_mill_time: i64) {
        self.player1
            .update_opt_timeout_status(curr_timestamp, poem_mill_time);
        self.player2
            .update_opt_timeout_status(curr_timestamp, poem_mill_time);
    }

    fn gc_update_to_json(&self) -> Option<String> {
        let gc_update_game = proto::GCUpdateGame {
            game_id: self.id.clone(),
            player1_id: self.player1.player_id.clone(),
            player1_name: self.player1.player_name.clone(),
            player1_next_opt_index: self.player1.next_opt_index,
            player1_opt_bitmap: self.player1.opt_bitmap,
            player2_id: self.player2.player_id.clone(),
            player2_name: self.player2.player_name.clone(),
            player2_next_opt_index: self.player2.next_opt_index,
            player2_opt_bitmap: self.player2.opt_bitmap,
        };

        return proto::ProtoData::gc_to_json_string(proto::PROTO_GCUPDATEGAME, gc_update_game);
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

        return proto::ProtoData::gc_to_json_string(proto::PROTO_GCENDGAME, gc_end_game);
    }
}

pub struct MatchGameController {
    game_map: HashMap<String, Game>,
    ended_game: Vec<String>,
    poem_table: PoemTable,
    petable: PETable,
    robot_ctrl: RobotController,
    tx: std::sync::mpsc::Sender<RedisOpt>,
    poem_mill_time: i64,
    poem_score: u32,
}

impl MatchGameController {
    pub fn new(
        tx: std::sync::mpsc::Sender<RedisOpt>,
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
            game.update_robot_opt(curr_timestamp, self.poem_mill_time, self.poem_score);
            game.update_opt_timeout_status(curr_timestamp, self.poem_mill_time);
            game.update_end_status();

            if game.is_dirty() {
                log::info!("Game {} data is dirty!", game.id);
                if let Some(proto_json_str) = game.gc_update_to_json() {
                    log::info!("Sync GCUpdateGame {} data -> Client!", game.id);
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
                log::info!("Game {} is END!", game.id);
                self.ended_game.push(game.id.clone());

                if let Some(proto_json_str) = game.gc_end_game_to_json(&self.petable) {
                    log::info!("Sync GCEndGame {} END data -> Client!", game.id);
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
                if let Ok(()) = self.tx.send(RedisOpt::GamePlayerData(
                    format!("{}_{}", &game.player1.player_id, &game.player1.player_name),
                    game.player1.player_level,
                )) {
                    // data send ok
                    log::info!(
                        "Player1 {}, name: {}, level {}, data is Sync to Redis",
                        game.player1.player_id,
                        game.player1.player_name,
                        game.player1.player_level
                    );
                } else {
                    // Channel send msg error
                    log::error!("Send player1 name level to Redis failed!");
                }
                // 将第2个玩家的数据也发到另一线程，存Redis
                if let Ok(()) = self.tx.send(RedisOpt::GamePlayerData(
                    format!("{}_{}", &game.player2.player_id, &game.player2.player_name),
                    game.player2.player_level,
                )) {
                    // data send ok
                    log::info!(
                        "Player2 {}, name: {}, level {}, data is Sync to Redis",
                        game.player2.player_id,
                        game.player2.player_name,
                        game.player2.player_level
                    );
                } else {
                    // Channel send msg error
                    log::error!("Send player2 name level to Redis failed!");
                }
            }
        }

        for game_id in self.ended_game.iter() {
            log::info!("Remove Ended Game: {}", game_id);
            if let Some(game) = self.game_map.remove(game_id) {
                log::info!("Game {} has Removed!", game.id);
            }
        }

        let game_num = self.game_map.len();
        if let Ok(()) = self.tx.send(RedisOpt::GameStatus(game_num as u32)) {}

        return some_signal_vec;
    }

    pub fn start_new_game(
        &mut self,
        mut player1: Player,
        mut player2: Player,
        curr_timestamp: i64,
    ) -> Option<Signal> {
        player1.next_opt_index = 0;
        player1.next_opt_timeout_timestamp = curr_timestamp + self.poem_mill_time + 1;
        player2.next_opt_index = 0;
        player2.next_opt_timeout_timestamp = curr_timestamp + self.poem_mill_time + 1;

        log::info!("Try Start a new Game!");
        let player_level = player1.player_level;
        if let Some(poem_json_str) = self
            .poem_table
            .get_random_game_data(player_level, MATCH_POEM_NUM)
        {
            let player1_id = player1.player_id.clone();
            let player1_name = player1.player_name.clone();
            let player2_id = player2.player_id.clone();
            let player2_name = player2.player_name.clone();

            let game_id = format!(
                "{}_{}_{}",
                player1.player_id.clone(),
                player2.player_id.clone(),
                curr_timestamp
            );

            let gc_start_game = proto::GCStartGame {
                game_id: game_id,
                player1_id: player1_id,
                player1_name: player1_name,
                player2_id: player2_id,
                player2_name: player2_name,
                poem_data_str: poem_json_str,
            };

            // 创建消息同步 Signal
            if let Some(gc_start_game_json_str) =
                proto::ProtoData::gc_to_json_string(proto::PROTO_GCSTARTGAME, gc_start_game)
            {
                let signal = Signal::Sync(
                    player1.endpoint_id.clone(),
                    player2.endpoint_id.clone(),
                    gc_start_game_json_str,
                );

                let game = Game::new(player1, player2, curr_timestamp);
                self.game_map.insert(game.id.clone(), game);

                let game_num = self.game_map.len();
                if let Ok(()) = self.tx.send(RedisOpt::GameStatus(game_num as u32)) {}

                return Some(signal);
            } else {
                log::error!("创建GCStartGame消息时Json序列化失败，无法进行游戏!");
            }
        } else {
            log::error!("诗词数据生成失败，无法进行游戏!");
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
        let player = Player {
            endpoint_id: None,
            player_id: robot.id.clone(),
            player_name: robot.name.clone(),
            player_level: robot.level,
            player_elo_score: robot.elo_score,
            player_correct_rate: robot.correct_rate,
            game_start_timestamp: curr_timestamp,
            next_opt_index: 0,
            next_opt_timeout_timestamp: -1,
            opt_bitmap: 0,
            is_dirty: false,
            robot: Some(robot),
            game_score: 0,
        };
        log::info!("ROBOT player created: {:?}", player);
        return player;
    }

    pub fn game_count(&self) -> usize {
        self.game_map.len()
    }
}

pub fn create_player_from_match(match_reqeust: MatchRequest, curr_timestamp: i64) -> Player {
    let player = Player {
        endpoint_id: match_reqeust.endpoint_id,
        player_id: match_reqeust.player_id,
        player_name: match_reqeust.player_name,
        player_level: match_reqeust.player_level,
        player_elo_score: match_reqeust.player_elo_score,
        player_correct_rate: match_reqeust.player_correct_rate,
        game_start_timestamp: curr_timestamp,
        next_opt_index: 0,
        next_opt_timeout_timestamp: -1,
        opt_bitmap: 0,
        is_dirty: false,
        robot: None,
        game_score: 0,
    };
    log::info!("Real Player Created: {:?}", player);
    return player;
}
