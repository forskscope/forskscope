use std::fs::{read_to_string, File};
use std::io::Read;

use chardetng::EncodingDetector;

use super::types::ReadContent;

const UTF8_CHARSET: &str = "UTF-8";
const NOT_TEXTFILE_CHARSET: &str = "(bytes array)";

pub fn filepath_content(filepath: &str) -> ReadContent {
    match read_to_string(filepath) {
        Ok(x) => {
            return ReadContent {
                charset: UTF8_CHARSET.to_owned(),
                content: x,
            }
        }
        Err(_) => (),
    };

    let mut file = File::open(filepath).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let mut detector = EncodingDetector::new();
    detector.feed(&buffer, true);
    let encoding = detector.guess(None, false);
    let (decoded, _, had_errors) = encoding.decode(&buffer);
    if !had_errors {
        ReadContent {
            charset: encoding.name().to_owned(),
            content: decoded.to_string(),
        }
    } else {
        const BYTES_ARRAY_ROW_LENGTH: usize = 16;
        let mut grid = String::new();
        for chunk in buffer.chunks(BYTES_ARRAY_ROW_LENGTH) {
            for byte in chunk {
                grid.push_str(&format!("{:02X} ", byte));
            }
            grid.push_str("\n");
        }
        ReadContent {
            charset: NOT_TEXTFILE_CHARSET.to_owned(),
            content: grid,
        }
    }
}
