use std::{
    fs::{read, read_to_string},
    path::PathBuf,
};

pub fn filepath_content(filepath: &str) -> String {
    let filepath = PathBuf::from(filepath);
    match read_to_string(&filepath) {
        Ok(x) => x,
        // todo
        Err(_) => {
            let read_as_bytes = read(&filepath).expect(
                format!(
                    "Failed to read as text file: {}",
                    &filepath.to_str().unwrap_or_default()
                )
                .as_str(),
            );
            read_as_bytes
                .iter()
                .map(|x| format!("{} ", x.to_string()))
                .collect::<String>()
                .trim_end()
                .to_owned()
        }
    }
}
