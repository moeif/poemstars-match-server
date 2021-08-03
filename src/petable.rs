use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PERecord {
    pub dmin: u32,
    pub dmax: u32,
    pub ea: f64,
    pub eb: f64,
    pub group: u32,
}

pub struct PETable {
    pub pe_vec: Vec<PERecord>,
}

impl PETable {
    pub fn new() -> Self {
        let file = std::fs::File::open("./configs/pet.csv").unwrap();
        let mut rdr = csv::Reader::from_reader(file);
        let mut pe_vec: Vec<PERecord> = Vec::new();
        for result in rdr.deserialize() {
            let record: PERecord = result.unwrap();
            pe_vec.push(record);
        }

        Self { pe_vec }
    }

    pub fn get_ea_eb(&self, elo1_score: u32, elo2_score: u32) -> (f64, f64, u32) {
        let diff_score = if elo1_score > elo2_score {
            elo1_score - elo2_score
        } else {
            elo2_score - elo1_score
        };
        let mut ea = -1.0f64;
        let mut eb = -1.0f64;
        let mut group = 0;
        for record in self.pe_vec.iter() {
            if diff_score >= record.dmin && diff_score <= record.dmax {
                ea = record.ea;
                eb = record.eb;
                group = record.group;
                break;
            }
        }

        // 如果没有在表中找到，则说明超出范围了
        if ea < 0.0 && eb < 0.0 {
            ea = 1.0;
            eb = 0.0;
            group = 9;
        }

        (ea, eb, group)
    }
}
