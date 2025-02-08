use similar::{DiffTag, TextDiff};

use super::types::{CharsDiff, LinesDiff};

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
                        chars_diff_if_replace: None,
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
                        chars_diff_if_replace: None,
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
                        chars_diff_if_replace: None,
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
                    let chars_diff_if_replace = TextDiff::configure()
                        .diff_chars(old_str.as_str(), new_str.as_str())
                        .ops()
                        .iter()
                        .map(|x| {
                            let diff_kind = x.tag();
                            match diff_kind {
                                DiffTag::Equal => {
                                    let old_range = x.old_range();
                                    let str = &old_str[old_range.start..old_range.end];
                                    CharsDiff {
                                        diff_kind,
                                        old_str: str.to_owned(),
                                        new_str: str.to_owned(),
                                    }
                                }
                                DiffTag::Delete => {
                                    let old_range = x.old_range();
                                    let str = &old_str[old_range.start..old_range.end];
                                    CharsDiff {
                                        diff_kind,
                                        old_str: str.to_owned(),
                                        new_str: "".to_owned(),
                                    }
                                }
                                DiffTag::Insert => {
                                    let new_range = x.new_range();
                                    let str = &new_str[new_range.start..new_range.end];
                                    CharsDiff {
                                        diff_kind,
                                        old_str: "".to_owned(),
                                        new_str: str.to_owned(),
                                    }
                                }
                                DiffTag::Replace => {
                                    let old_range = x.old_range();
                                    let new_range = x.new_range();
                                    let old_str =
                                        (&old_str[old_range.start..old_range.end]).to_owned();
                                    let new_str =
                                        (&new_str[new_range.start..new_range.end]).to_owned();
                                    CharsDiff {
                                        diff_kind,
                                        old_str,
                                        new_str,
                                    }
                                }
                            }
                        })
                        .collect::<Vec<CharsDiff>>();

                    LinesDiff {
                        diff_kind,
                        lines_count,
                        old_lines,
                        new_lines,
                        chars_diff_if_replace: Some(chars_diff_if_replace),
                    }
                }
            }
        })
        .collect::<Vec<LinesDiff>>()
}
