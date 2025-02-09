use similar::{DiffTag, TextDiff};

use super::types::{LinesDiff, ReplaceCharsDiff, ReplaceLineDiff};

pub fn lines_diff(old_content: &str, new_content: &str) -> Vec<LinesDiff> {
    let diff = TextDiff::configure().diff_lines(old_content, new_content);
    let old_lines: Vec<String> = old_content.lines().map(|line| line.to_owned()).collect();
    let new_lines: Vec<String> = new_content.lines().map(|line| line.to_owned()).collect();
    diff.ops()
        .iter()
        .map(|x| {
            let diff_kind = x.tag();
            match diff_kind {
                DiffTag::Equal => {
                    let old_range = x.old_range();
                    let lines_count = old_range.end - old_range.start + 1;
                    let lines = old_lines[old_range.start..old_range.end].to_vec();
                    LinesDiff {
                        diff_kind,
                        lines_count,
                        old_lines: lines.to_owned(),
                        new_lines: lines,
                        replace_diff_lines: None,
                    }
                }
                DiffTag::Delete => {
                    let old_range = x.old_range();
                    let lines_count = old_range.end - old_range.start + 1;
                    let old_lines = old_lines[old_range.start..old_range.end].to_vec();
                    LinesDiff {
                        diff_kind,
                        lines_count,
                        old_lines,
                        new_lines: vec![],
                        replace_diff_lines: None,
                    }
                }
                DiffTag::Insert => {
                    let new_range = x.new_range();
                    let lines_count = new_range.end - new_range.start + 1;
                    let new_lines = new_lines[new_range.start..new_range.end].to_vec();
                    LinesDiff {
                        diff_kind,
                        lines_count,
                        old_lines: vec![],
                        new_lines,
                        replace_diff_lines: None,
                    }
                }
                DiffTag::Replace => {
                    let old_range = x.old_range();
                    let new_range = x.new_range();
                    let old_lines_count = old_range.end - old_range.start + 1;
                    let new_lines_count = new_range.end - new_range.start + 1;
                    let lines_count = if old_lines_count < new_lines_count {
                        new_lines_count
                    } else {
                        old_lines_count
                    };
                    let old_lines = old_lines[old_range.start..old_range.end].to_vec();
                    let new_lines = new_lines[new_range.start..new_range.end].to_vec();

                    let old_str = old_lines.join("\n");
                    let new_str = new_lines.join("\n");
                    let replace_diff_chars = TextDiff::configure()
                        .diff_chars(old_str.as_str(), new_str.as_str())
                        .ops()
                        .iter()
                        .flat_map(|x| {
                            let diff_kind = x.tag();
                            match diff_kind {
                                DiffTag::Equal => {
                                    let old_range = x.old_range();
                                    let str = &old_str[old_range.start..old_range.end];
                                    let lines = str.split("\n");
                                    lines
                                        .enumerate()
                                        .flat_map(|(i, x)| {
                                            let mut ret: Vec<ReplaceCharsDiff> = vec![];
                                            if 0 < i {
                                                ret.push(ReplaceCharsDiff {
                                                    diff_kind: None,
                                                    old_str: "".to_owned(),
                                                    new_str: "".to_owned(),
                                                })
                                            }
                                            ret.push(ReplaceCharsDiff {
                                                diff_kind: Some(diff_kind),
                                                old_str: x.to_owned(),
                                                new_str: x.to_owned(),
                                            });
                                            ret
                                        })
                                        .collect::<Vec<ReplaceCharsDiff>>()
                                }
                                DiffTag::Delete => {
                                    let old_range = x.old_range();
                                    let str = &old_str[old_range.start..old_range.end];
                                    let lines = str.split("\n");
                                    lines
                                        .enumerate()
                                        .flat_map(|(i, x)| {
                                            let mut ret: Vec<ReplaceCharsDiff> = vec![];
                                            if 0 < i {
                                                ret.push(ReplaceCharsDiff {
                                                    diff_kind: None,
                                                    old_str: "".to_owned(),
                                                    new_str: "".to_owned(),
                                                })
                                            }
                                            ret.push(ReplaceCharsDiff {
                                                diff_kind: Some(diff_kind),
                                                old_str: x.to_owned(),
                                                new_str: "".to_owned(),
                                            });
                                            ret
                                        })
                                        .collect::<Vec<ReplaceCharsDiff>>()
                                }
                                DiffTag::Insert => {
                                    let new_range = x.new_range();
                                    let str = &new_str[new_range.start..new_range.end];
                                    let lines = str.split("\n");
                                    lines
                                        .enumerate()
                                        .flat_map(|(i, x)| {
                                            let mut ret: Vec<ReplaceCharsDiff> = vec![];
                                            if 0 < i {
                                                ret.push(ReplaceCharsDiff {
                                                    diff_kind: None,
                                                    old_str: "".to_owned(),
                                                    new_str: "".to_owned(),
                                                })
                                            }
                                            ret.push(ReplaceCharsDiff {
                                                diff_kind: Some(diff_kind),
                                                old_str: "".to_owned(),
                                                new_str: x.to_owned(),
                                            });
                                            ret
                                        })
                                        .collect::<Vec<ReplaceCharsDiff>>()
                                }
                                DiffTag::Replace => {
                                    let old_range = x.old_range();
                                    let new_range = x.new_range();
                                    let old_str =
                                        (&old_str[old_range.start..old_range.end]).to_owned();
                                    let new_str =
                                        (&new_str[new_range.start..new_range.end]).to_owned();
                                    let old_lines = old_str.split("\n");
                                    let new_lines = new_str.split("\n");
                                    let lines =
                                        if old_lines.clone().count() < new_lines.clone().count() {
                                            new_lines.clone()
                                        } else {
                                            old_lines.clone()
                                        };
                                    lines
                                        .into_iter()
                                        .enumerate()
                                        .flat_map(|(i, _x)| {
                                            let old_lines = old_lines.clone();
                                            let new_lines = new_lines.clone();

                                            let mut ret: Vec<ReplaceCharsDiff> = vec![];
                                            if 0 < i {
                                                ret.push(ReplaceCharsDiff {
                                                    diff_kind: None,
                                                    old_str: "".to_owned(),
                                                    new_str: "".to_owned(),
                                                })
                                            }
                                            let old_line =
                                                old_lines.clone().nth(i).unwrap_or_default();
                                            let new_line =
                                                new_lines.clone().nth(i).unwrap_or_default();
                                            ret.push(ReplaceCharsDiff {
                                                diff_kind: Some(diff_kind),
                                                old_str: old_line.to_owned(),
                                                new_str: new_line.to_owned(),
                                            });
                                            ret
                                        })
                                        .collect::<Vec<ReplaceCharsDiff>>()
                                }
                            }
                        })
                        .collect::<Vec<ReplaceCharsDiff>>();
                    let mut replace_diff_lines: Vec<ReplaceLineDiff> = vec![];
                    let mut replace_diff_line: Vec<ReplaceCharsDiff> = vec![];
                    replace_diff_chars.iter().for_each(|x| {
                        if x.diff_kind.is_none() {
                            replace_diff_lines.push(ReplaceLineDiff {
                                chars_diff: replace_diff_line.clone(),
                            });
                            replace_diff_line = vec![];
                        } else {
                            replace_diff_line.push(ReplaceCharsDiff {
                                diff_kind: x.diff_kind,
                                old_str: x.old_str.clone(),
                                new_str: x.new_str.clone(),
                            })
                        }
                    });
                    if 0 < replace_diff_line.len() {
                        replace_diff_lines.push(ReplaceLineDiff {
                            chars_diff: replace_diff_line,
                        });
                    }

                    LinesDiff {
                        diff_kind,
                        lines_count,
                        old_lines,
                        new_lines,
                        replace_diff_lines: Some(replace_diff_lines),
                    }
                }
            }
        })
        .collect::<Vec<LinesDiff>>()
}
