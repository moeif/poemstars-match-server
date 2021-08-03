use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PoemRecord {
    pub level_id: u32,
    pub poem_id: u32,
    pub q_sign: u32,
    pub a_sign1: u32,
    pub a_sign2: u32,
    pub a_sign3: u32,
    pub a_sign4: u32,
}

pub struct PoemTable {
    // 每一个关卡id，对应着一首诗，
    pub level_map: HashMap<u32, Vec<PoemRecord>>,
    pub count: u32,
    // 玩家等级-哪些关卡可以选择
    pub level_vec_map: HashMap<u32, Vec<u32>>,
}

impl PoemTable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/poem.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);

        let mut level_map = HashMap::new();

        let mut sum = 0;
        for result in rdr.deserialize() {
            let record: PoemRecord = result.unwrap();
            if !level_map.contains_key(&record.level_id) {
                level_map.insert(record.level_id, Vec::new());
            }

            if let Some(poem_vec) = level_map.get_mut(&record.level_id) {
                poem_vec.push(record);
            }
            sum += 1;
        }

        // 设置不同玩家的等级在匹配玩法中可以从哪些关卡中随机生成
        let mut id_vec_map: HashMap<u32, Vec<u32>> = HashMap::new();
        id_vec_map.insert(0, (0..=20).collect());
        id_vec_map.insert(11, (100..=300).collect());
        id_vec_map.insert(21, (200..=400).collect());
        id_vec_map.insert(31, (300..=500).collect());
        id_vec_map.insert(41, (400..=600).collect());
        id_vec_map.insert(51, (500..=700).collect());
        id_vec_map.insert(61, (600..=800).collect());
        id_vec_map.insert(71, (300..=sum).collect());

        Self {
            level_map,
            count: sum,
            level_vec_map: id_vec_map,
        }
    }

    pub fn get_random_game_data(&mut self, level: u32, count: u32) -> Option<String> {
        let key = match level {
            0..=10 => 0,
            11..=20 => 11,
            21..=30 => 21,
            31..=40 => 31,
            41..=50 => 41,
            51..=60 => 51,
            61..=70 => 61,
            _ => 71,
        };
        let mut rng = rand::thread_rng();
        let mut selected_poem_record: Vec<&PoemRecord> = Vec::new();

        if let Some(ref mut level_id_vec) = self.level_vec_map.get_mut(&key) {
            shuffle_vec(level_id_vec);
            for i in 0..count as usize {
                let level_id = level_id_vec[i];

                if let Some(poem_vec) = self.level_map.get(&level_id) {
                    let peom_count = poem_vec.len();
                    let selection_index = rng.gen_range(0..peom_count);
                    let poem_record = &poem_vec[selection_index];
                    selected_poem_record.push(poem_record);
                }
            }

            if let Ok(json_str) = serde_json::to_string(&selected_poem_record) {
                return Some(json_str);
            }
        } else {
            println!("逻辑错误");
        }

        return None;
    }
}

fn shuffle_vec(list: &mut Vec<u32>) {
    let mut rng = rand::thread_rng();
    // let score_offset: i32 = rng.gen_range(-32..33);
    let last = list.len() - 1;
    for i in last..=0 {
        let selection_index = rng.gen_range(0..i + 1);
        let tmp = list[i];
        list[i] = list[selection_index];
        list[selection_index] = tmp;
    }
}
