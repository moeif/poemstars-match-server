use crate::petable::PETable;
use crate::proto;
use crate::robot::Robot;
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

    pub fn get_match_req2_id_name(&self) -> Option<(String, String)> {
        if let Some(ref match_req2) = self.match_req2 {
            return Some((
                match_req2.cg_match_info.id.clone(),
                match_req2.cg_match_info.name.clone(),
            ));
        };
        return None;
    }

    pub fn get_match_req1_endpoint_id(&self) -> Option<String> {
        return Some(self.match_req1.endpoint_id.clone());
    }

    pub fn get_match_req2_endpoint_id(&self) -> Option<String> {
        if let Some(ref match_req2) = self.match_req2 {
            return Some(match_req2.endpoint_id.clone());
        }

        return None;
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
        println!("更新匹配: {}", curr_timestamp);
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
                        let robot = Robot::new(
                            player1_match_req.cg_match_info.elo_score,
                            player1_match_req.cg_match_info.correct_rate,
                        );
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
