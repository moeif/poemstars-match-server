use rand::Rng;

const MIN_CORRECTION_PERCENT: f64 = 20.0;
const MAX_CORRECTION_PERCENT: f64 = 90.0;

pub struct Robot {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub elo_score: u32,
    pub correct_rate: f64,
    pub next_opt_wait_time: i64, // 下一次操作等待时间
}

impl Robot {
    pub fn set_next_opt_wait_time(&mut self, max_time: i64) {
        let cut_time = max_time / 3;
        let min_opt_time = max_time - cut_time;
        let max_opt_time = max_time;
        let mut rng = rand::thread_rng();
        self.next_opt_wait_time = rng.gen_range(min_opt_time..max_opt_time);
    }

    pub fn get_opt_result(&self) -> u32 {
        let mut rng = rand::thread_rng();
        let rand_value = rng.gen_range(0.0..100.0);
        if rand_value >= 100.0 - self.correct_rate {
            return 0;
        } else {
            return 1;
        }
    }
}

pub struct RobotController {}

impl RobotController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_robot(
        &self,
        competitor_level: u32,
        competitor_elo_score: u32,
        competitor_correct_rate: f64,
        max_opt_wait_time: i64,
    ) -> Robot {
        let relative_elo_score = competitor_elo_score as i32;
        let mut rng = rand::thread_rng();
        let score_offset: i32 = rng.gen_range(-32..33);
        let my_score = if score_offset >= 0 {
            relative_elo_score + score_offset
        } else {
            if relative_elo_score - score_offset >= 0 {
                relative_elo_score - score_offset
            } else {
                relative_elo_score
            }
        };

        let mut robot = Robot {
            id: "tmp_robot_id".to_string(),
            name: "tmp_robot_name".to_string(),
            level: competitor_level,
            elo_score: my_score as u32,
            correct_rate: if competitor_correct_rate < MIN_CORRECTION_PERCENT {
                MIN_CORRECTION_PERCENT
            } else if competitor_correct_rate > MAX_CORRECTION_PERCENT {
                MAX_CORRECTION_PERCENT
            } else {
                competitor_correct_rate
            },
            next_opt_wait_time: -1,
        };
        robot.set_next_opt_wait_time(max_opt_wait_time);
        return robot;
    }
}
