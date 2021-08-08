use rand::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoemLineRecord {
    pub level_id: u32,
    pub poem_id: u32,
    pub q_sign: u32,
    pub a_sign1: u32,
    pub a_sign2: u32,
    pub a_sign3: u32,
    pub a_sign4: u32,
}

pub struct PoemRecord {
    pub line_sign_map: HashMap<u32, Vec<PoemLineRecord>>,
    pub line_sign_vec: Vec<u32>,
}

impl PoemRecord {
    pub fn new() -> Self {
        Self {
            line_sign_map: HashMap::new(),
            line_sign_vec: Vec::new(),
        }
    }

    pub fn add_line_record(&mut self, line_record: PoemLineRecord) {
        if !self.line_sign_map.contains_key(&line_record.q_sign) {
            self.line_sign_map.insert(line_record.q_sign, Vec::new());
            self.line_sign_vec.push(line_record.q_sign);
        }

        if let Some(sign_vec) = self.line_sign_map.get_mut(&line_record.q_sign) {
            sign_vec.push(line_record);
        }
    }

    pub fn get_random_line_record(&mut self) -> Option<PoemLineRecord> {
        let mut rng = rand::thread_rng();
        self.line_sign_vec.shuffle(&mut rng);
        let selected_line_sign = self.line_sign_vec[0];

        if let Some(line_record_vec) = self.line_sign_map.get(&selected_line_sign) {
            let len = line_record_vec.len();
            let rand_index = rng.gen_range(0..len);
            return Some(line_record_vec[rand_index].clone());
        }

        return None;
    }
}

pub struct PoemTable {
    // 每一个关卡id，对应着一首诗，
    pub level_map: HashMap<u32, PoemRecord>,
    pub count: u32,
    // 玩家等级-哪些关卡可以选择
    pub level_vec_map: HashMap<u32, Vec<u32>>,
}

impl PoemTable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/poem.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);

        let mut level_map: HashMap<u32, PoemRecord> = HashMap::new();

        let mut sum = 0;
        for result in rdr.deserialize() {
            let line_record: PoemLineRecord = result.unwrap();
            let level_id = line_record.level_id;

            if let Some(ref mut poem_record) = level_map.get_mut(&level_id) {
                poem_record.add_line_record(line_record);
            } else {
                let mut poem_record = PoemRecord::new();
                poem_record.add_line_record(line_record);
                level_map.insert(level_id, poem_record);
                sum += 1;
            }
        }

        // 设置不同玩家的等级在匹配玩法中可以从哪些关卡中随机生成
        let mut id_vec_map: HashMap<u32, Vec<u32>> = HashMap::new();
        id_vec_map.insert(1, (1..=20).collect());
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

    pub fn get_random_game_data(&mut self, level: u32, count: u32) -> Option<Vec<PoemLineRecord>> {
        //Option<String> {
        let key = match level {
            0..=10 => 1,
            11..=20 => 11,
            21..=30 => 21,
            31..=40 => 31,
            41..=50 => 41,
            51..=60 => 51,
            61..=70 => 61,
            _ => 71,
        };
        let mut selected_poem_record: Vec<PoemLineRecord> = Vec::new();
        let mut rng = rand::thread_rng();
        if let Some(ref mut level_id_vec) = self.level_vec_map.get_mut(&key) {
            level_id_vec.shuffle(&mut rng);

            for i in 0..count {
                let level_id = level_id_vec[i as usize];
                if let Some(poem_record) = self.level_map.get_mut(&level_id) {
                    if let Some(random_line_record) = poem_record.get_random_line_record() {
                        selected_poem_record.push(random_line_record);
                    } else {
                        println!("Logic Error, can not generate random poem data!");
                    }
                } else {
                    println!("Get PoemRecord Failed: {}", level_id);
                }
            }

            // if let Ok(json_str) = serde_json::to_string(&selected_poem_record) {
            //     return Some(json_str);
            // }

            if selected_poem_record.len() > 0 {
                return Some(selected_poem_record);
            }
        } else {
            println!("逻辑错误");
        }

        return None;
    }
}
