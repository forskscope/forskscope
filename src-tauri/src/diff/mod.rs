use similar::{DiffTag, TextDiff};

use serde::{Deserialize, Serialize};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::fs::{read, read_to_string};
use std::path::Path;

#[derive(Deserialize)]
enum DiffRequestType {
    Content,
    Filepath,
}
#[derive(Deserialize)]
pub struct DiffRequest {
    diff_request_type: DiffRequestType,
    content: String,
}

#[derive(Serialize)]
pub struct DiffBlockOp {
    tag: DiffTag,
    lines: Vec<String>,
    new_lines_num: usize,
    diff_block_index: usize,
}

#[derive(Serialize)]
pub struct DiffResponse {
    old_blocks: Vec<DiffBlockOp>,
    new_blocks: Vec<DiffBlockOp>,
    diff_blocks_num: usize,
}

#[tauri::command]
pub fn diff(old_diff_request: DiffRequest, new_diff_request: DiffRequest) -> DiffResponse {
    let old_content = read_content(&old_diff_request);
    let new_content = read_content(&new_diff_request);
    diff_contents(old_content.as_str(), new_content.as_str())
}

fn read_content(diff_request: &DiffRequest) -> String {
    match diff_request.diff_request_type {
        DiffRequestType::Content => diff_request.content.to_owned(),
        DiffRequestType::Filepath => {
            match read_to_string(diff_request.content.as_str()) {
                Ok(x) => x,
                // todo
                Err(_) => {
                    let read_as_bytes = read(diff_request.content.as_str()).expect(
                        format!(
                            "Failed to read as text file: {}",
                            diff_request.content.as_str()
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
    }
}

#[tauri::command]
pub fn path_join(path1: &str, path2: &str) -> String {
    let path1 = Path::new(path1);
    let path2 = Path::new(path2);
    path1
        .join(path2)
        .into_os_string()
        .into_string()
        .unwrap_or_else(|oss| oss.to_string_lossy().into_owned())
}

#[derive(Serialize)]
pub struct ListDirReponse {
    current_dir: String,
    dirs: Vec<String>,
    files: Vec<String>,
}
#[tauri::command]
pub fn list_dir(current_dir: &str) -> ListDirReponse {
    let current_dir = if current_dir.is_empty() {
        std::env::current_dir()
            .expect("Failed to get current directory")
            .canonicalize()
            .expect("Failed to canonicalize path")
            .display()
            .to_string()
    } else {
        current_dir.to_owned()
    };
    let mut dirs = Vec::<String>::new();
    let mut files = Vec::<String>::new();
    let read = std::fs::read_dir(current_dir.as_str())
        .expect(format!("Failed to read {}", current_dir).as_str());
    for x in read {
        match x {
            Ok(dir_entry) => {
                let name = dir_entry.file_name().to_string_lossy().to_string();
                match dir_entry.file_type() {
                    Ok(file_type) => {
                        if file_type.is_dir() {
                            dirs.push(name)
                        } else {
                            files.push(name)
                        }
                    }
                    _ => {}
                }
            }
            Err(err) => println!("Failed to get dir/file info due to {}", err),
        }
    }
    ListDirReponse {
        current_dir: current_dir,
        dirs: dirs,
        files: files,
    }
}

fn diff_contents(old_content: &str, new_content: &str) -> DiffResponse {
    let diff = TextDiff::configure().diff_lines(old_content, new_content);
    let old_lines = old_content.lines().collect::<Vec<&str>>();
    let new_lines = new_content.lines().collect::<Vec<&str>>();
    let mut old_blocks = Vec::<DiffBlockOp>::new();
    let mut new_blocks = Vec::<DiffBlockOp>::new();
    let mut diff_blocks_num = 0;
    diff.ops().iter().for_each(|x| {
        let tag = x.tag();
        match tag {
            DiffTag::Equal => {
                let old_range = x.old_range();
                let str = old_lines[old_range.start..old_range.end]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                old_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: str.clone(),
                    new_lines_num: 0,
                    diff_block_index: 0,
                });
                new_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: str.clone(),
                    new_lines_num: 0,
                    diff_block_index: 0,
                });
            }
            DiffTag::Delete => {
                let old_range = x.old_range();
                let old_str = old_lines[old_range.start..old_range.end]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                let new_lines_num = old_range.end - old_range.start;
                old_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: old_str,
                    new_lines_num: 0,
                    diff_block_index: diff_blocks_num,
                });
                new_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: Vec::new(),
                    new_lines_num: new_lines_num,
                    diff_block_index: diff_blocks_num,
                });
                diff_blocks_num += 1;
            }
            DiffTag::Insert => {
                let new_range = x.new_range();
                let new_str = new_lines[new_range.start..new_range.end]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                let new_lines_num = new_range.end - new_range.start;
                old_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: Vec::new(),
                    new_lines_num: new_lines_num,
                    diff_block_index: diff_blocks_num,
                });
                new_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: new_str,
                    new_lines_num: 0,
                    diff_block_index: diff_blocks_num,
                });
                diff_blocks_num += 1;
            }
            DiffTag::Replace => {
                let old_range = x.old_range();
                let old_str = old_lines[old_range.start..old_range.end]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                let new_range = x.new_range();
                let new_str = new_lines[new_range.start..new_range.end]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                let old_str_lines_num = old_range.end - old_range.start;
                let new_str_lines_num = new_range.end - new_range.start;
                let new_lines_nums = match old_str_lines_num.cmp(&new_str_lines_num) {
                    Equal => (0, 0),
                    Less => (old_str_lines_num.abs_diff(new_str_lines_num), 0),
                    Greater => (0, old_str_lines_num.abs_diff(new_str_lines_num)),
                };
                old_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: old_str,
                    new_lines_num: new_lines_nums.0,
                    diff_block_index: diff_blocks_num,
                });
                new_blocks.push(DiffBlockOp {
                    tag: tag,
                    lines: new_str,
                    new_lines_num: new_lines_nums.1,
                    diff_block_index: diff_blocks_num,
                });
                diff_blocks_num += 1;
            }
        }
    });
    DiffResponse {
        old_blocks: old_blocks,
        new_blocks: new_blocks,
        diff_blocks_num: diff_blocks_num,
    }
}
