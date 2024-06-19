use similar::{TextDiff, DiffTag};

use serde::Serialize;
use std::cmp::Ordering::{Equal, Less, Greater};

// todo
// const S: &str = "Oh a charming cat!\nWow, nice.\nGood day.";
// const T: &str = "A great dog.\n\nWow, nice.\nBetter day.";
use std::sync::OnceLock;
fn s() -> String {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| {
        let ret = std::fs::read_to_string("./Cargo.lock").unwrap();
        format!("{}\n{}", ret.split("\n").take(30).collect::<Vec<_>>().join("\n"), ret.split("\n").take(2).collect::<Vec<_>>().join("\n"))
    }).to_owned()
}
fn t() -> String {
    static T: OnceLock<String> = OnceLock::new();
    T.get_or_init(|| {
        let ret = std::fs::read_to_string("./Cargo.lock").unwrap();
        format!("{}\n\n{}", ret.split("\n").take(3).collect::<Vec<_>>().join("\n"), ret.split("\n").take(40).collect::<Vec<_>>().join("\n"))
    }).to_owned()
}

#[derive(Serialize)]
pub struct DiffBlockOp {
    tag: DiffTag,
    lines: Vec<String>,
    new_lines_num: usize,
}

#[tauri::command]
pub fn diff(_old_filepath: &str, _new_filepath: &str) -> (Vec::<DiffBlockOp>, Vec::<DiffBlockOp>) {
    // todo
    let s = s().to_owned();
    let t = t().to_owned();

    let diff = TextDiff::configure().diff_lines(s.as_str(), t.as_str());
    let old_lines = s.lines().collect::<Vec<&str>>();
    let new_lines = t.lines().collect::<Vec<&str>>();
    let mut old_blocks = Vec::<DiffBlockOp>::new();
    let mut new_blocks = Vec::<DiffBlockOp>::new();
    diff.ops().iter().for_each(|x| {
        let tag = x.tag();
        match tag {
            DiffTag::Equal => {
                let old_range = x.old_range();
                let str = old_lines[old_range.start..old_range.end].iter().map(|x| x.to_string()).collect::<Vec<_>>();
                old_blocks.push(DiffBlockOp{ tag: tag, lines: str.clone(), new_lines_num: 0 });
                new_blocks.push(DiffBlockOp{ tag: tag, lines: str.clone(), new_lines_num: 0 });
            },
            DiffTag::Delete => {
                let old_range = x.old_range();
                let old_str = old_lines[old_range.start..old_range.end].iter().map(|x| x.to_string()).collect::<Vec<_>>();
                let new_lines_num = old_range.end - old_range.start;
                old_blocks.push(DiffBlockOp{ tag: tag, lines: old_str, new_lines_num: 0 });
                new_blocks.push(DiffBlockOp{ tag: tag, lines: Vec::new(), new_lines_num: new_lines_num });
            },
            DiffTag::Insert => {
                let new_range = x.new_range();
                let new_str = new_lines[new_range.start..new_range.end].iter().map(|x| x.to_string()).collect::<Vec<_>>();
                let new_lines_num = new_range.end - new_range.start;
                old_blocks.push(DiffBlockOp{ tag: tag, lines: Vec::new(), new_lines_num: new_lines_num });
                new_blocks.push(DiffBlockOp{ tag: tag, lines: new_str, new_lines_num: 0 });
            },
            DiffTag::Replace => {
                let old_range = x.old_range();
                let old_str = old_lines[old_range.start..old_range.end].iter().map(|x| x.to_string()).collect::<Vec<_>>();
                let new_range = x.new_range();
                let new_str = new_lines[new_range.start..new_range.end].iter().map(|x| x.to_string()).collect::<Vec<_>>();
                let old_str_lines_num = old_range.end - old_range.start;
                let new_str_lines_num = new_range.end - new_range.start;
                let new_lines_nums = match old_str_lines_num.cmp(&new_str_lines_num) {
                    Equal => (0, 0),
                    Less => (old_str_lines_num.abs_diff(new_str_lines_num), 0),
                    Greater => (0, old_str_lines_num.abs_diff(new_str_lines_num)),
                };
                old_blocks.push(DiffBlockOp{ tag: tag, lines: old_str, new_lines_num: new_lines_nums.0 });
                new_blocks.push(DiffBlockOp{ tag: tag, lines: new_str, new_lines_num: new_lines_nums.1 });
            },
        }
    });
    (old_blocks, new_blocks)
}

#[tauri::command]
pub fn file_content(filepath: &str) -> String {
    // todo
    (if filepath.is_empty() { s() } else { t() }).to_owned()
}

#[tauri::command]
pub fn list_dir(dirpath: &str) -> Vec<Vec<String>> {
    let dirpath = if dirpath.is_empty() { "." } else { dirpath };
    let mut dirs = Vec::<String>::new();
    let mut files = Vec::<String>::new();
    for x in std::fs::read_dir(dirpath).unwrap() {
        let dir_entry = x.unwrap();
        let name = dir_entry.file_name().to_string_lossy().to_string();
        match dir_entry.file_type() {
            Ok(file_type) => if file_type.is_dir() { dirs.push(name) } else { files.push(name)},
            _ => {}
        }
    }
    Vec::from([dirs, files])
}
