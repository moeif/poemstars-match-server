use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RobotRecord {
    pub id: String,
    pub name: String,
}

pub struct RobotTable {
    pub pe_vec: Vec<PERecord>,
}

impl RobotTable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/robot.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);
        let mut pe_vec: Vec<RobotRecord> = Vec::new();
        for result in rdr.deserialize() {
            let record: RobotRecord = result.unwrap();
            pe_vec.push(record);
        }

        Self { pe_vec }
    }
}
