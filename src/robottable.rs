use rand::Rng;
use serde::Deserialize;
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RobotRecord {
    pub id: String,
    pub name: String,
}

pub struct RobotTable {
    pub robot_vec: VecDeque<RobotRecord>,
}

impl RobotTable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/robot_info.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);
        let mut robot_vec = VecDeque::new();
        for result in rdr.deserialize() {
            let record: RobotRecord = result.unwrap();
            robot_vec.push_back(record);
        }

        Self { robot_vec }
    }

    pub fn get_id_name(&mut self) -> (String, String) {
        if let Some(robot) = self.robot_vec.pop_front() {
            (robot.id, robot.name)
        } else {
            let mut rng = rand::thread_rng();
            let num: i32 = rng.gen_range(100000..999990);
            let id = Uuid::new_v4().to_simple().to_string();
            let name = format!("Player{}", num);

            (id, name)
        }
    }

    pub fn back_id_name(&mut self, id: String, name: String) {
        let robot = RobotRecord { id, name };

        self.robot_vec.push_front(robot);
    }
}
