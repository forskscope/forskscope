use similar::{DiffTag, TextDiff};

use super::{
    str::{multibyte_str_byte_indices, split_lines_with_endings},
    types::{CharsDiff, CharsDiffLines, LinesDiff},
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
