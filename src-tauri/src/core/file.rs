use std::fs::File;
use std::io::Read;

use chardetng::EncodingDetector;

use super::types::ReadContent;

const UTF8_CHARSET: &str = "UTF-8";
const NOT_TEXTFILE_CHARSET: &str = "(bytes array)";

pub fn filepath_content(filepath: &str) -> ReadContent {
    let mut file = File::open(filepath).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let is_binary = buffer.windows(2).any(|window| window[0] == 0x00);
    if is_binary {
        const BYTES_ARRAY_ROW_LENGTH: usize = 16;
        let mut grid = String::new();
        for chunk in buffer.chunks(BYTES_ARRAY_ROW_LENGTH) {
            for byte in chunk {
                grid.push_str(&format!("{:02X} ", byte));
            }
            grid.push_str("\n");
        }
        return ReadContent {
            charset: NOT_TEXTFILE_CHARSET.to_owned(),
            content: grid,
        };
    }

    match std::str::from_utf8(&buffer) {
        Ok(x) => {
            return ReadContent {
                charset: UTF8_CHARSET.to_owned(),
                content: x.to_owned(),
            }
        }
        Err(_) => (),
    }

    let mut detector = EncodingDetector::new();
    detector.feed(&buffer, true);
    let encoding = detector.guess(None, false);
    let (decoded, _, had_errors) = encoding.decode(&buffer);
    if had_errors {
        eprint!("not binary, not utf-8 text and not any other encoded text.")
    }
    ReadContent {
        charset: encoding.name().to_owned(),
        content: decoded.to_string(),
    }
}
