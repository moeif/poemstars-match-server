use crate::petable::PETable;

pub struct MatchRequest {
    pub endpoint_id: Option<String>,
    pub player_id: String,
    pub player_name: String,
    pub player_level: u32,
    pub player_elo_score: u32,
    pub player_correct_rate: f64,
    pub timestamp: i64,
}

pub struct MatchController {
    match_vec: Vec<MatchRequest>,
    last_update_timestamp: i64,
    pe_table: PETable,
}

impl MatchController {
    pub fn new() -> Self {
        Self {
            last_update_timestamp: -1,
            pe_table: PETable::new(),
            match_vec: Vec::new(),
        }
    }

    pub fn add_match(&mut self, match_request: MatchRequest) {
        self.match_vec.push(match_request);
    }

    pub fn update_matches(
        &mut self,
        curr_timestamp: i64,
    ) -> Option<(Option<MatchRequest>, Option<MatchRequest>)> {
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
                        let player1_elo_score = match_req.player_elo_score;
                        let player2_elo_score = check_req.player_elo_score;
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
                let waited_time = curr_timestamp - match_req.timestamp;
                let mut use_robot = false;
                let mut matched = false;

                // 找到一个潜在的对手后，根据当前玩家等待的时间判断，对手是否满足要求
                if min_index > 0 {
                    if let Some(check_req) = self.match_vec.get(min_index as usize) {
                        let (_ea, _eb, group) = self
                            .pe_table
                            .get_ea_eb(match_req.player_elo_score, check_req.player_elo_score);

                        if waited_time <= 1000 && group <= 0 {
                            // 完美匹配
                            matched = true;
                        } else if waited_time <= 2500 && group <= 1 {
                            // 匹配成功
                            matched = true;
                        } else if waited_time <= 3500 && group <= 2 {
                            // 匹配成功
                            matched = true;
                        } else if waited_time <= 4500 && group <= 3 {
                            matched = true;
                        } else {
                            if group <= 4 {
                                matched = true;
                            }
                        }
                    }
                } else {
                    // 没有找到合适的潜在对手，也就是没有玩家了，判断一下时间，超时则使用机器人
                    if waited_time > 4500 {
                        matched = true;
                        use_robot = true;
                    }
                }

                if matched {
                    if use_robot {
                        let player1_match_req = self.match_vec.remove(imain);
                        return Some((Some(player1_match_req), None));
                    } else {
                        // 如果 remove 失败，则会 Panic。因为是从前往后检查的，所以要先移除后面的
                        let player2_match_req = self.match_vec.remove(min_index as usize);
                        let player1_match_req = self.match_vec.remove(imain);
                        return Some((Some(player1_match_req), Some(player2_match_req)));
                    }
                }
            }
        }

        return None;
    }
}
