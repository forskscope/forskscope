//! Directory-level comparison report: `DirComparisonReport`, `BatchSummary` (RFC-008).

use std::fmt::Write as _;
use std::path::Path;
use crate::dir::{BatchManifest, RecEntry, RecStatus};
use super::file::{ReportOptions, ReportPathMode};

// ── Directory comparison report ───────────────────────────────────────────────

/// A rendered report for a directory comparison.
#[derive(Debug, Clone)]
pub struct DirComparisonReport {
    pub left_display:  String,
    pub right_display: String,
    pub total:         usize,
    pub equal:         usize,
    pub changed:       usize,
    pub left_only:     usize,
    pub right_only:    usize,
    pub symlinks:      usize,
    pub file_rows:     Vec<DirFileRow>,
    pub batch_summary: Option<BatchSummary>,
    pub options:       ReportOptions,
}

/// One file row in the directory report.
#[derive(Debug, Clone)]
pub struct DirFileRow {
    pub path:        String,
    pub status:      String,
    pub left_size:   Option<u64>,
    pub right_size:  Option<u64>,
}

/// Summary of a batch copy operation (from `BatchManifest`).
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub operation_id: String,
    pub succeeded:    usize,
    pub failed:       usize,
}

impl DirComparisonReport {
    /// Build a directory comparison report.
    ///
    /// `left_root` / `right_root` are used for path display when
    /// `path_mode` is `Relative`. Pass `None` to fall back to `NameOnly`.
    pub fn from_entries(
        entries:    &[RecEntry],
        left_root:  Option<&Path>,
        right_root: Option<&Path>,
        manifest:   Option<&BatchManifest>,
        opts:       ReportOptions,
    ) -> Self {
        let left_display  = super::display_path(left_root,  &opts.path_mode, None);
        let right_display = super::display_path(right_root, &opts.path_mode, None);

        let mut equal = 0usize; let mut changed = 0usize;
        let mut left_only = 0usize; let mut right_only = 0usize;
        let mut symlinks = 0usize;

        let file_rows: Vec<DirFileRow> = entries.iter().map(|e| {
            let status = match e.status {
                RecStatus::Equal     => { equal     += 1; "equal".into() }
                RecStatus::Changed   => { changed   += 1; "modified".into() }
                RecStatus::LeftOnly  => { left_only += 1; "left only".into() }
                RecStatus::RightOnly => { right_only += 1; "right only".into() }
                RecStatus::Symlink   => { symlinks  += 1; "symlink".into() }
                RecStatus::Computing => "computing".into(),
            };
            let path = match opts.path_mode {
                ReportPathMode::NameOnly => e.rel_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| e.rel_path.display().to_string()),
                _ => e.rel_path.display().to_string(),
            };
            DirFileRow { path, status, left_size: e.left_size, right_size: e.right_size }
        }).collect();

        let batch_summary = manifest.map(|m| BatchSummary {
            operation_id: m.operation_id.to_string(),
            succeeded:    m.succeeded(),
            failed:       m.failed(),
        });

        let total = entries.len();
        Self {
            left_display, right_display, total, equal, changed,
            left_only, right_only, symlinks, file_rows, batch_summary, options: opts,
        }
    }

    // ── Markdown ──────────────────────────────────────────────────────────────

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "# ForskScope Directory Comparison Report\n");
        let _ = writeln!(s, "## Summary\n");
        let _ = writeln!(s, "| Field | Count |");
        let _ = writeln!(s, "|---|---:|");
        let _ = writeln!(s, "| Left  | `{}` |", self.left_display);
        let _ = writeln!(s, "| Right | `{}` |", self.right_display);
        let _ = writeln!(s, "| Total entries | {} |", self.total);
        let _ = writeln!(s, "| Equal         | {} |", self.equal);
        let _ = writeln!(s, "| Modified      | {} |", self.changed);
        let _ = writeln!(s, "| Left only     | {} |", self.left_only);
        let _ = writeln!(s, "| Right only    | {} |", self.right_only);
        if self.symlinks > 0 {
            let _ = writeln!(s, "| Symlinks      | {} |", self.symlinks);
        }
        let _ = writeln!(s);

        // Only include non-equal rows in the report by default.
        let interesting: Vec<&DirFileRow> = self.file_rows.iter()
            .filter(|r| r.status != "equal")
            .collect();

        if !interesting.is_empty() {
            let _ = writeln!(s, "## Changed Files\n");
            if self.options.include_sizes {
                let _ = writeln!(s, "| Path | Status | Left size | Right size |");
                let _ = writeln!(s, "|---|---|---:|---:|");
                for r in &interesting {
                    let ls = r.left_size.map(fmt_size).unwrap_or_else(|| "—".into());
                    let rs = r.right_size.map(fmt_size).unwrap_or_else(|| "—".into());
                    let _ = writeln!(s, "| `{}` | {} | {} | {} |", r.path, r.status, ls, rs);
                }
            } else {
                let _ = writeln!(s, "| Path | Status |");
                let _ = writeln!(s, "|---|---|");
                for r in &interesting {
                    let _ = writeln!(s, "| `{}` | {} |", r.path, r.status);
                }
            }
            let _ = writeln!(s);
        }

        if let Some(bs) = &self.batch_summary {
            let _ = writeln!(s, "## Batch Operation\n");
            let _ = writeln!(s, "- Operation ID: `{}`", bs.operation_id);
            let _ = writeln!(s, "- Succeeded: {}", bs.succeeded);
            let _ = writeln!(s, "- Failed: {}", bs.failed);
            let _ = writeln!(s);
        }

        s
    }

    // ── JSON ──────────────────────────────────────────────────────────────────

    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"schema_version\": 1,");
        let _ = writeln!(s, "  \"app_version\": {:?},", env!("CARGO_PKG_VERSION"));
        let _ = writeln!(s, "  \"kind\": \"directory_comparison\",");
        let _ = writeln!(s, "  \"summary\": {{");
        let _ = writeln!(s, "    \"left\": {:?},",      self.left_display);
        let _ = writeln!(s, "    \"right\": {:?},",     self.right_display);
        let _ = writeln!(s, "    \"total\": {},",        self.total);
        let _ = writeln!(s, "    \"equal\": {},",        self.equal);
        let _ = writeln!(s, "    \"changed\": {},",      self.changed);
        let _ = writeln!(s, "    \"left_only\": {},",    self.left_only);
        let _ = writeln!(s, "    \"right_only\": {},",   self.right_only);
        let _ = writeln!(s, "    \"symlinks\": {}",      self.symlinks);
        let _ = writeln!(s, "  }},");

        // changed files
        let _ = writeln!(s, "  \"files\": [");
        let interesting: Vec<&DirFileRow> = self.file_rows.iter()
            .filter(|r| r.status != "equal")
            .collect();
        for (i, r) in interesting.iter().enumerate() {
            let comma = if i + 1 < interesting.len() { "," } else { "" };
            let ls = r.left_size.map(|n| n.to_string()).unwrap_or_else(|| "null".into());
            let rs = r.right_size.map(|n| n.to_string()).unwrap_or_else(|| "null".into());
            let _ = writeln!(s, "    {{ \"path\": {:?}, \"status\": {:?}, \"left_bytes\": {}, \"right_bytes\": {} }}{}",
                r.path, r.status, ls, rs, comma);
        }
        let _ = writeln!(s, "  ]");

        if let Some(bs) = &self.batch_summary {
            let _ = writeln!(s, "  ,\"batch\": {{");
            let _ = writeln!(s, "    \"operation_id\": {:?},", bs.operation_id);
            let _ = writeln!(s, "    \"succeeded\": {},", bs.succeeded);
            let _ = writeln!(s, "    \"failed\": {}", bs.failed);
            let _ = writeln!(s, "  }}");
        }

        let _ = write!(s, "}}");
        s
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────



fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    }
}
