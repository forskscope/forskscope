use std::fs::{read, read_to_string};

pub fn filepath_content(filepath: &str) -> String {
    match read_to_string(filepath) {
        Ok(x) => x,
        // todo
        Err(_) => {
            let read_as_bytes = read(&filepath)
                .expect(format!("Failed to read as text file: {}", filepath).as_str());
            read_as_bytes
                .iter()
                .map(|x| format!("{} ", x.to_string()))
                .collect::<String>()
                .trim_end()
                .to_owned()
        }
    }
}
