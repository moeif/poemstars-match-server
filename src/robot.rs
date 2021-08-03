use rand::Rng;

pub struct Robot {
    pub elo_score: u32,
}

impl Robot {
    pub fn new(relative_elo_score: u32) -> Self {
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
            elo_score: my_score as u32,
        }
    }
}
