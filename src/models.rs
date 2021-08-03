use crate::common::Signal;
use crate::petable::PETable;
use crate::proto;
use crate::robot::Robot;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

const MATCH_POEM_NUM: i32 = 10;
const MATCH_TIME: u8 = 5;

// 匹配结果
pub struct MatchResult {
    pub match_req1: MatchingReq,
    pub match_req2: Option<MatchingReq>,
    pub ea: f64,
    pub eb: f64,
    pub use_robot: bool,
    pub robot: Option<Robot>,
}

impl MatchResult {
    pub fn new(
        match_req1: MatchingReq,
        match_req2: Option<MatchingReq>,
        ea: f64,
        eb: f64,
        use_robot: bool,
        robot: Option<Robot>,
    ) -> Self {
        Self {
            match_req1,
            match_req2,
            ea,
            eb,
            use_robot,
            robot,
        }
    }
}

pub struct MatchingReq {
    pub endpoint_id: String,
    pub cg_match_info: proto::CGStartMatch,
    pub start_match_timestamp: i64,
}

pub struct MatchQueue {
    queue: VecDeque<MatchingReq>,
}

impl MatchQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn front_match_timestamp(&self) -> i64 {
        if let Some(item) = self.queue.front() {
            return item.start_match_timestamp;
        }
        return -1;
    }

    pub fn add_match(&mut self, matching_req: MatchingReq) {
        self.queue.push_back(matching_req);
    }

    pub fn get_front(&mut self) -> Option<MatchingReq> {
        self.queue.pop_front()
    }

    pub fn match_count(&self) -> u32 {
        self.queue.len() as u32
    }
}

pub struct MatchController {
    max_range: u32,
    queue_map: HashMap<u32, MatchQueue>,
    match_vec: Vec<MatchingReq>,
    last_update_timestamp: i64,
    pe_table: PETable,
}

impl MatchController {
    pub fn new() -> Self {
        Self {
            max_range: 0,
            queue_map: HashMap::new(),
            last_update_timestamp: -1,
            pe_table: PETable::new(),
            match_vec: Vec::new(),
        }
    }

    pub fn add_match(&mut self, matching_req: MatchingReq) {
        self.match_vec.push(matching_req);
    }

    pub fn update_matches(&mut self, curr_timestamp: i64) -> Option<MatchResult> {
        self.last_update_timestamp = curr_timestamp;

        let len = self.match_vec.len();
        if len == 0 {
            return None;
        }

        for imain in 0..(len - 1) {
            let mut min_index = -1;
            let mut min_score_diff = 0;
            if let Some(match_req) = self.match_vec.get(imain) {
                // 找到 elo_score 相差最小的另一个玩家
                for icheck in (imain + 1)..len {
                    if let Some(check_req) = self.match_vec.get(icheck) {
                        let player1_elo_score = match_req.cg_match_info.elo_score;
                        let player2_elo_score = check_req.cg_match_info.elo_score;
                        let diff = if player1_elo_score > player2_elo_score {
                            player1_elo_score - player2_elo_score
                        } else {
                            player2_elo_score - player1_elo_score
                        };

                        if min_index < 0 || diff < min_score_diff {
                            min_index = icheck as i32;
                            min_score_diff = diff;
                        }
                    }
                }

                // 先判断有没有匹配超时
                let waited_time = curr_timestamp as f64 - match_req.start_match_timestamp as f64;
                let mut use_robot = false;
                let mut matched = false;
                let mut ea = 0.0;
                let mut eb = 0.0;

                // 找到一个潜在的对手后，根据当前玩家等待的时间判断，对手是否满足要求
                if min_index > 0 {
                    if let Some(check_req) = self.match_vec.get(min_index as usize) {
                        let (_ea, _eb, group) = self.pe_table.get_ea_eb(
                            match_req.cg_match_info.elo_score,
                            check_req.cg_match_info.elo_score,
                        );

                        ea = _ea;
                        eb = _eb;

                        if waited_time <= 1.0 && group <= 0 {
                            // 完美匹配
                            matched = true;
                        } else if waited_time <= 2.5 && group <= 1 {
                            // 匹配成功
                            matched = true;
                        } else if waited_time <= 3.5 && group <= 2 {
                            // 匹配成功
                            matched = true;
                        } else if waited_time <= 4.5 && group <= 3 {
                            matched = true;
                        } else {
                            if group <= 4 {
                                matched = true;
                            }
                        }
                    }
                } else {
                    // 没有找到合适的潜在对手，也就是没有玩家了，判断一下时间，超时则使用机器人
                    if waited_time > 4.5 {
                        matched = true;
                        use_robot = true;
                    }
                }

                if matched {
                    if use_robot {
                        let player1_match_req = self.match_vec.remove(imain);
                        let robot = Robot::new(player1_match_req.cg_match_info.elo_score);
                        let (_ea, _eb, group) = self
                            .pe_table
                            .get_ea_eb(player1_match_req.cg_match_info.elo_score, robot.elo_score);

                        let match_result = MatchResult::new(
                            player1_match_req,
                            None,
                            _ea,
                            _eb,
                            use_robot,
                            Some(robot),
                        );
                        return Some(match_result);
                    } else {
                        // 如果 remove 失败，则会 Panic。因为是从前往后检查的，所以要先移除后面的
                        let player2_match_req = self.match_vec.remove(min_index as usize);
                        let player1_match_req = self.match_vec.remove(imain);
                        let match_result = MatchResult::new(
                            player1_match_req,
                            Some(player2_match_req),
                            ea,
                            eb,
                            use_robot,
                            None,
                        );

                        return Some(match_result);
                    }
                }
            }
        }

        return None;
    }
}

// ----------- game -----------
#[derive(Serialize)]
pub struct MatchPlayer {
    pub id: String,
    pub endpoint_id: String,
    pub name: String,
    pub last_opt_timestamp: i64, // 最后一次的操作时间
    pub last_opt_index: i32,     // 最后一次操作的索引
    pub opt_bitmap: u32,         // 操作位数据, 0 正确，1 错误
    pub is_dirty: bool,
}

impl MatchPlayer {
    fn new(endpoint_id: String, id: String, name: String, last_opt_timestamp: i64) -> Self {
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
        self.last_opt_index + 1 == MATCH_POEM_NUM
    }

    fn on_opt(&mut self, opt: proto::CGMatchGameOpt, curr_timestamp: i64) {
        if self.last_opt_index + 1 == opt.opt_index as i32 {
            self.last_opt_index += 1;
            self.opt_bitmap |= opt.opt_result << self.last_opt_index;
        }
        self.is_dirty = true;
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64) {
        self.last_opt_index += 1;
        self.opt_bitmap |= 1 << self.last_opt_index;
        self.is_dirty = true;
    }

    fn is_dirty(&mut self) -> bool {
        let tmp_is_dirty = self.is_dirty;
        self.is_dirty = false;
        return tmp_is_dirty;
    }
}

#[derive(Serialize)]
pub struct MatchGame {
    pub id: String,           // 游戏ID
    pub start_timestamp: i64, // 游戏开始时间戳
    pub player1: MatchPlayer,
    pub player2: MatchPlayer,
    pub is_gaming: bool, // 游戏进行中
    pub is_dirty: bool,
}

impl MatchGame {
    fn new(player1: MatchPlayer, player2: MatchPlayer, start_timestamp: i64) -> Self {
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
    game_map: HashMap<String, MatchGame>,
    last_update_timestamp: i64,
    ended_game: Vec<String>,
}

impl MatchGameController {
    pub fn new() -> Self {
        Self {
            game_map: HashMap::new(),
            last_update_timestamp: -1,
            ended_game: Vec::new(),
        }
    }

    pub fn on_opt(&mut self, opt_info: proto::CGMatchGameOpt, curr_timestamp: i64) {
        if let Some(game) = self.game_map.get_mut(&opt_info.game_id) {
            game.on_opt(opt_info, curr_timestamp);
        }
    }

    pub fn update_games(&mut self, curr_timestamp: i64) -> Option<Vec<Signal>> {
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

    pub fn create_new_game(&self, match_result: MatchResult) -> Option<Signal> {
        None
    }
}
