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
    pub robot_deque_array: [VecDeque<RobotRecord>; 4],
    pub get_index: usize,
    pub back_index: usize,
}

impl RobotTable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/robot_info.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);

        let mut robot_deque_array: [VecDeque<RobotRecord>; 4] = [
            VecDeque::new(),
            VecDeque::new(),
            VecDeque::new(),
            VecDeque::new(),
        ];

        let mut record_index = 0;

        for result in rdr.deserialize() {
            let record: RobotRecord = result.unwrap();
            let deque_index = record_index % robot_deque_array.len();
            robot_deque_array[deque_index].push_back(record);
            record_index += 1;
        }

        Self {
            robot_deque_array,
            get_index: 0,
            back_index: 0,
        }
    }

    pub fn get_id_name(&mut self) -> (String, String) {
        if let Some(robot) =
            self.robot_deque_array[self.get_index % self.robot_deque_array.len()].pop_front()
        {
            self.get_index += 1;
            (robot.id, robot.name)
        } else {
            self.get_index += 1;
            let mut rng = rand::thread_rng();
            let num: i32 = rng.gen_range(100000..999990);
            let id = Uuid::new_v4().to_simple().to_string();
            let name = format!("Player{}", num);

            (id, name)
        }
    }

    pub fn back_id_name(&mut self, id: String, name: String) {
        let robot = RobotRecord { id, name };
        self.robot_deque_array[self.back_index % self.robot_deque_array.len()].push_front(robot);
        self.back_index += 1;
    }
}
