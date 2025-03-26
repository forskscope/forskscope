use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use similar::{DiffTag, TextDiff};

use super::{
    file::{arg_to_filepath, validate_filepath},
    str::{multibyte_str_byte_indices, split_lines_with_endings},
    types::{CharsDiff, CharsDiffLines, CompareSetItem, LinesDiff},
};

pub fn lines_diffs(old_content: &str, new_content: &str) -> Vec<LinesDiff> {
    let diff = TextDiff::configure().diff_lines(old_content, new_content);
    let old_lines: Vec<String> = split_lines_with_endings(old_content);
    let new_lines: Vec<String> = split_lines_with_endings(new_content);
    diff.ops()
        .iter()
        .enumerate()
        .map(|(diff_index, x)| {
            let diff_kind = x.tag();
            match diff_kind {
                DiffTag::Equal => {
                    let old_range = x.old_range();
                    let lines_count = old_range.end - old_range.start;
                    let lines = old_lines[old_range.start..old_range.end].to_vec();
                    LinesDiff {
                        diff_index,
                        diff_kind,
                        lines_count,
                        old_lines: lines.to_owned(),
                        new_lines: lines,
                    }
                }
                DiffTag::Delete => {
                    let old_range = x.old_range();
                    let lines_count = old_range.end - old_range.start;
                    let old_lines = old_lines[old_range.start..old_range.end].to_vec();
                    LinesDiff {
                        diff_index,
                        diff_kind,
                        lines_count,
                        old_lines,
                        new_lines: vec![],
                    }
                }
                DiffTag::Insert => {
                    let new_range = x.new_range();
                    let lines_count = new_range.end - new_range.start;
                    let new_lines = new_lines[new_range.start..new_range.end].to_vec();
                    LinesDiff {
                        diff_index,
                        diff_kind,
                        lines_count,
                        old_lines: vec![],
                        new_lines,
                    }
                }
                DiffTag::Replace => {
                    let old_range = x.old_range();
                    let new_range = x.new_range();

                    let old_lines_count = old_range.end - old_range.start;
                    let new_lines_count = new_range.end - new_range.start;
                    let lines_count = if old_lines_count < new_lines_count {
                        new_lines_count
                    } else {
                        old_lines_count
                    };
                    let old_lines = old_lines[old_range.start..old_range.end].to_vec();
                    let new_lines = new_lines[new_range.start..new_range.end].to_vec();

                    LinesDiff {
                        diff_index,
                        diff_kind: diff_kind.to_owned(),
                        lines_count,
                        old_lines,
                        new_lines,
                    }
                }
            }
        })
        .collect::<Vec<LinesDiff>>()
}

pub fn chars_diffs(lines_diffs: &Vec<LinesDiff>) -> Vec<CharsDiffLines> {
    lines_diffs
        .iter()
        .map(|x| {
            let old_str = x.old_lines.join("");
            let new_str = x.new_lines.join("");
            chars_diff(x.diff_index, old_str.as_str(), new_str.as_str())
        })
        .collect()
}

fn chars_diff(diff_index: usize, old_str: &str, new_str: &str) -> CharsDiffLines {
    let mut old_lines: Vec<Vec<CharsDiff>> = vec![];
    let mut new_lines: Vec<Vec<CharsDiff>> = vec![];

    let mut old_line: Vec<CharsDiff> = vec![];
    let mut new_line: Vec<CharsDiff> = vec![];

    let mut old_chars = String::new();
    let mut new_chars = String::new();

    TextDiff::configure()
        .algorithm(similar::Algorithm::Lcs)
        .diff_chars(old_str, new_str)
        .ops()
        .iter()
        .for_each(|x| {
            let diff_kind = x.tag();
            match diff_kind {
                DiffTag::Equal => {
                    let old_range = x.old_range();
                    // let str = &old_str[old_range.start..old_range.end];
                    let (start, end) =
                        multibyte_str_byte_indices(old_str, old_range.start, old_range.end)
                            .unwrap();
                    let str = &old_str[start..end];
                    str.chars().for_each(|x| {
                        old_chars.push(x);
                        new_chars.push(x);
                        if x == '\n' || x == '\r' {
                            if 0 < old_chars.len() {
                                old_line.push(CharsDiff {
                                    diff_kind,
                                    chars: old_chars.clone(),
                                });
                                old_chars = String::new();
                                new_line.push(CharsDiff {
                                    diff_kind,
                                    chars: new_chars.clone(),
                                });
                                new_chars = String::new();
                            }
                            old_lines.push(old_line.clone());
                            old_line = vec![];
                            new_lines.push(new_line.clone());
                            new_line = vec![];
                        }
                    });
                    if 0 < old_chars.len() {
                        old_line.push(CharsDiff {
                            diff_kind,
                            chars: old_chars.clone(),
                        });
                        old_chars = String::new();
                        new_line.push(CharsDiff {
                            diff_kind,
                            chars: new_chars.clone(),
                        });
                        new_chars = String::new();
                    }
                }
                DiffTag::Delete => {
                    let old_range = x.old_range();
                    // let str = &old_str[old_range.start..old_range.end];
                    let (start, end) =
                        multibyte_str_byte_indices(old_str, old_range.start, old_range.end)
                            .unwrap();
                    let str = &old_str[start..end];
                    str.chars().for_each(|x| {
                        old_chars.push(x);
                        if x == '\n' || x == '\r' {
                            if 0 < old_chars.len() {
                                old_line.push(CharsDiff {
                                    diff_kind,
                                    chars: old_chars.clone(),
                                });
                                old_chars = String::new();
                            }
                            old_lines.push(old_line.clone());
                            old_line = vec![];
                        }
                    });
                    if 0 < old_chars.len() {
                        old_line.push(CharsDiff {
                            diff_kind,
                            chars: old_chars.clone(),
                        });
                        old_chars = String::new();
                    }
                }
                DiffTag::Insert => {
                    let new_range = x.new_range();
                    // let str = &new_str[new_range.start..new_range.end];
                    let (start, end) =
                        multibyte_str_byte_indices(new_str, new_range.start, new_range.end)
                            .unwrap();
                    let str = &new_str[start..end];
                    str.chars().for_each(|x| {
                        new_chars.push(x);
                        if x == '\n' || x == '\r' {
                            if 0 < new_chars.len() {
                                new_line.push(CharsDiff {
                                    diff_kind,
                                    chars: new_chars.clone(),
                                });
                                new_chars = String::new();
                            }
                            new_lines.push(new_line.clone());
                            new_line = vec![];
                        }
                    });
                    if 0 < new_chars.len() {
                        new_line.push(CharsDiff {
                            diff_kind,
                            chars: new_chars.clone(),
                        });
                        new_chars = String::new();
                    }
                }
                DiffTag::Replace => {
                    let old_range = x.old_range();
                    // let old_str = (&old_str[old_range.start..old_range.end]).to_owned();
                    let (old_start, old_end) =
                        multibyte_str_byte_indices(old_str, old_range.start, old_range.end)
                            .unwrap();
                    let old_str = &old_str[old_start..old_end];
                    old_str.chars().for_each(|x| {
                        old_chars.push(x);
                        if x == '\n' || x == '\r' {
                            if 0 < old_chars.len() {
                                old_line.push(CharsDiff {
                                    diff_kind,
                                    chars: old_chars.clone(),
                                });
                                old_chars = String::new();
                            }
                            old_lines.push(old_line.clone());
                            old_line = vec![];
                        }
                    });
                    if 0 < old_chars.len() {
                        old_line.push(CharsDiff {
                            diff_kind,
                            chars: old_chars.clone(),
                        });
                        old_chars = String::new();
                    }

                    let new_range = x.new_range();
                    // let new_str = (&new_str[new_range.start..new_range.end]).to_owned();
                    let (new_start, new_end) =
                        multibyte_str_byte_indices(new_str, new_range.start, new_range.end)
                            .unwrap();
                    let new_str = &new_str[new_start..new_end];
                    new_str.chars().for_each(|x| {
                        new_chars.push(x);
                        if x == '\n' || x == '\r' {
                            if 0 < new_chars.len() {
                                new_line.push(CharsDiff {
                                    diff_kind,
                                    chars: new_chars.clone(),
                                });
                                new_chars = String::new();
                            }
                            new_lines.push(new_line.clone());
                            new_line = vec![];
                        }
                    });
                    if 0 < new_chars.len() {
                        new_line.push(CharsDiff {
                            diff_kind,
                            chars: new_chars.clone(),
                        });
                        new_chars = String::new();
                    }
                }
            }
        });
    if 0 < old_line.len() {
        old_lines.push(old_line);
    }
    if 0 < new_line.len() {
        new_lines.push(new_line);
    }

    CharsDiffLines {
        diff_index,
        old_lines,
        new_lines,
    }
}

/// digest comparison around files
pub fn file_digest_diff(filename: &str, old_dir: &str, new_dir: &str) -> Result<bool, String> {
    let old_filepath = Path::new(old_dir).join(filename);
    let new_filepath = Path::new(new_dir).join(filename);
    filepaths_digest_diff(&old_filepath, &new_filepath)
}

/// digest comparison around directories
pub fn dir_digest_diff(dirname: &str, old_dir: &str, new_dir: &str) -> Result<bool, String> {
    let old_dirpath = Path::new(old_dir).join(dirname);
    let new_dirpath = Path::new(new_dir).join(dirname);
    dirpaths_digest_diff(&old_dirpath, &new_dirpath)
}

/// decide comparison mode
pub fn binary_comparison_only(filepath: &str) -> bool {
    match validate_filepath(filepath) {
        Some(x) => !x,
        None => false,
    }
}

/// get compare set item from startup args
pub fn startup_compare_set_item(filepath: &Option<OsString>) -> CompareSetItem {
    match arg_to_filepath(&filepath) {
        Some(filepath) => {
            let binary_comparison_only = binary_comparison_only(filepath.as_str());
            CompareSetItem {
                filepath,
                binary_comparison_only,
            }
        }
        None => CompareSetItem {
            filepath: String::new(),
            binary_comparison_only: false,
        },
    }
}

/// digest comparison around file paths
fn filepaths_digest_diff(old_filepath: &PathBuf, new_filepath: &PathBuf) -> Result<bool, String> {
    let old_metadata = fs::metadata(old_filepath).expect("Failed to get file metadata on old");
    let new_metadata = fs::metadata(new_filepath).expect("Failed to get file metadata on new");

    // compare file size
    if old_metadata.len() != new_metadata.len() {
        return Ok(false);
    }

    // compare file bytes
    let old_file = File::open(old_filepath).expect("Failed to open file on old");
    let new_file = File::open(new_filepath).expect("Failed to open file on new");

    let mut old_reader = BufReader::new(old_file);
    let mut new_reader = BufReader::new(new_file);

    // 8 KB buffer
    const COMPARISON_UNIT_BUFFER_SIZE: usize = 8192;
    let mut old_buffer = [0; COMPARISON_UNIT_BUFFER_SIZE];
    let mut new_buffer = [0; COMPARISON_UNIT_BUFFER_SIZE];

    loop {
        let old_bytes = old_reader
            .read(&mut old_buffer)
            .expect("Failed to read buffer on old");
        let new_bytes = new_reader
            .read(&mut new_buffer)
            .expect("Failed to read buffer on new");

        if old_bytes != new_bytes {
            return Ok(false);
        }

        if old_bytes == 0 {
            // EOF reached in both files
            break;
        }

        if old_buffer[..old_bytes] != new_buffer[..new_bytes] {
            return Ok(false);
        }
    }

    Ok(true)
}

/// digest comparison around directory paths
fn dirpaths_digest_diff(old_dirpath: &PathBuf, new_dirpath: &PathBuf) -> Result<bool, String> {
    let mut old_files = vec![];
    let mut old_dirs = vec![];
    let mut new_files = vec![];
    let mut new_dirs = vec![];

    let old_entries = fs::read_dir(old_dirpath).expect("Failed to read dir on old");
    let new_entries = fs::read_dir(new_dirpath).expect("Failed to read dir on new");

    for entry in old_entries {
        let entry = entry.expect("Failed to get entry on old read dir");
        let path = entry.path();
        if path.is_file() {
            old_files.push(path);
        } else if path.is_dir() {
            old_dirs.push(path);
        }
    }

    for entry in new_entries {
        let entry = entry.expect("Failed to get entry on new read dir");
        let path = entry.path();
        if path.is_file() {
            new_files.push(path);
        } else if path.is_dir() {
            new_dirs.push(path);
        }
    }

    old_files.sort();
    old_dirs.sort();
    new_files.sort();
    new_dirs.sort();

    // compare files
    if old_files.len() != new_files.len() || old_dirs.len() != new_dirs.len() {
        return Ok(false);
    }

    for (old_file, new_file) in old_files.iter().zip(new_files.iter()) {
        if old_file.file_name() != new_file.file_name() {
            return Ok(false);
        }
        if !filepaths_digest_diff(old_file, new_file).expect("Failed to compare files on read dir")
        {
            return Ok(false);
        }
    }

    // compare directories recursively
    for (subdir1, subdir2) in old_dirs.iter().zip(new_dirs.iter()) {
        if subdir1.file_name() != subdir2.file_name() {
            return Ok(false);
        }
        if !dirpaths_digest_diff(subdir1, subdir2)? {
            return Ok(false);
        }
    }

    Ok(true)
}
