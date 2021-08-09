use crate::robottable::RobotTable;
use rand::Rng;

const MIN_CORRECTION_PERCENT: f64 = 40.0;
const MAX_CORRECTION_PERCENT: f64 = 80.0;

#[derive(Debug)]
pub struct Robot {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub elo_score: u32,
    pub correct_rate: f64,
    pub next_early_opt_time: i64,
}

impl Robot {
    pub fn set_next_opt_wait_time(&mut self, max_time: i64) {
        let half_time = max_time / 2;
        let mut rng = rand::thread_rng();
        self.next_early_opt_time = rng.gen_range(1..half_time);
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

pub struct RobotController {
    robottable: RobotTable,
}

impl RobotController {
    pub fn new() -> Self {
        Self {
            robottable: RobotTable::new(),
        }
    }

    pub fn get_robot(
        &mut self,
        competitor_level: u32,
        competitor_elo_score: u32,
        competitor_correct_rate: f64,
        max_opt_wait_time: i64,
    ) -> Robot {
        let relative_elo_score = competitor_elo_score as i32;
        let mut rng = rand::thread_rng();
        let score_offset: i32 = rng.gen_range(-10..11);
        let my_score = if score_offset >= 0 {
            relative_elo_score + score_offset
        } else {
            if relative_elo_score - score_offset >= 0 {
                relative_elo_score - score_offset
            } else {
                relative_elo_score
            }
        };

        let (id, name) = self.robottable.get_id_name();

        let mut robot = Robot {
            id,
            name,
            level: competitor_level,
            elo_score: my_score as u32,
            correct_rate: rng.gen_range(MIN_CORRECTION_PERCENT..=MAX_CORRECTION_PERCENT),
            // correct_rate: if competitor_correct_rate == 0.0 {
            //     MIN_CORRECTION_PERCENT
            // } else if competitor_correct_rate < MIN_CORRECTION_PERCENT {
            //     MIN_CORRECTION_PERCENT
            // } else if competitor_correct_rate > MAX_CORRECTION_PERCENT {
            //     MAX_CORRECTION_PERCENT
            // } else {
            //     competitor_correct_rate
            // },
            next_early_opt_time: -1,
        };

        robot.set_next_opt_wait_time(max_opt_wait_time);
        return robot;
    }

    pub fn back_robot(&mut self, robot: Robot) {
        self.robottable.back_id_name(robot.id, robot.name);
    }
}
