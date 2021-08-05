use chrono::prelude::*;
use std::fs::OpenOptions;
use std::io::prelude::*;
// pub fn get_timestamp() -> i64 {
//     Utc::now().timestamp()
// }

pub fn get_timestamp_millis() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn append_lines(file: &str, content: &str) -> bool {
    if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(file) {
        file.write(content.as_bytes()).unwrap();
        file.write("\n".as_bytes()).unwrap();
        return true;
    }
    return false;
}
