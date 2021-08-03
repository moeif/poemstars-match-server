use rand::Rng;

const MIN_CORRECTION_PERCENT: f64 = 20.0;
const MAX_CORRECTION_PERCENT: f64 = 90.0;

pub struct Robot {
    pub id: String,
    pub name: String,
    pub elo_score: u32,
    pub correct_rate: f64,
}

impl Robot {
    pub fn new(relative_elo_score: u32, relative_correct_rate: f64) -> Self {
        let relative_elo_score = relative_elo_score as i32;
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

        Self {
            id: "tmp_robot_id".to_string(),
            name: "tmp_robot_name".to_string(),
            elo_score: my_score as u32,
            correct_rate: if relative_correct_rate < MIN_CORRECTION_PERCENT {
                MIN_CORRECTION_PERCENT
            } else if relative_correct_rate > MAX_CORRECTION_PERCENT {
                MAX_CORRECTION_PERCENT
            } else {
                relative_correct_rate
            },
        }
    }
}
