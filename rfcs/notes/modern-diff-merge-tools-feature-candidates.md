# ForskScope Competitive Diff/Merge Feature Analysis and Candidate RFC Addendum

Date: 2026-06-08  
Project direction: Dioxus + Rust core + `similar` v3 family + editor adapter  
Status: Addendum after RFC packages v0.1–v0.4 / RFC-001 through RFC-042 (roadmap originally numbered RFC-000)

---

## 1. Executive Summary

The current RFC set is complete enough to serve as the baseline implementation plan for the Dioxus migration. It covers core extraction, `similar` v3 integration, Dioxus shell, editor adapter, explorer, diff/merge workspaces, save safety, directory comparison, session persistence, encoding policy, large-file behavior, undo/redo, editor bridge security, parity testing, directory merge, atomic file operations, visual semantics, WebView compatibility, report export, preferences, external tools, user documentation, three-way merge, conflict resolution, live reload, scalable directory indexing, VCS context, patch workflow, editor verification, and v1 stabilization.

However, comparison with modern diff/merge tools reveals additional product-quality candidates that should be explicitly sorted into three groups:

1. **Default v1 features**: features that directly support Forskscope's identity as a local diff/merge workstation.
2. **Optional or plugin-like features**: valuable features that should not increase baseline complexity or safety risk.
3. **Non-goals**: features that would turn Forskscope into a Git client, IDE, cloud sync tool, or AI merge product.

The major recommendation is:

> ForskScope should borrow the best review and merge ergonomics from Git, VS Code, IntelliJ IDEA, Meld, KDiff3, and Beyond Compare, but should not become a Git GUI or IDE.

---

## 2. Compared Tools and Extracted Lessons

### 2.1 Git diff

Git provides the strongest lessons for diff semantics rather than GUI design:

- multiple diff algorithms or heuristics
- readable hunk boundaries
- moved-line highlighting
- whitespace error highlighting
- whitespace-aware moved-line handling
- patch generation/application conventions

Git's documentation exposes `diff.indentHeuristic`, `diff.algorithm`, `diff.colorMoved`, and `diff.colorMovedWS` settings. These are not merely command-line options; they represent mature user expectations around readable diffs and semantic noise reduction.

**Lesson for Forskscope:** expose diff behavior as named comparison profiles rather than many raw switches in the main UI.

Candidate default features:

- Ignore whitespace modes.
- Normalize line endings as a visible comparison option.
- Context line control and unchanged-fragment folding.
- Moved-line detection or moved-block visual hinting.
- Whitespace error highlighting.
- Patch export/apply compatible with common Git-style workflows.

Candidate optional features:

- Expose algorithm choice: default, Myers, patience-like, histogram-like if available or implemented.
- Expert profile editor for diff algorithms and whitespace treatment.

Non-goal:

- Reimplement Git internals or full Git history operations.

---

### 2.2 VS Code Git and merge views

VS Code provides strong lessons for Git-integrated review workflows:

- Source Control view separates unstaged and staged changes.
- Diff editor supports side-by-side and inline review.
- Gutter indicators show added, modified, and deleted lines.
- Inline diff preview can stage or revert specific changes.
- Partial staging supports selected lines/ranges.
- Merge editor uses Incoming, Current, and Result panes.
- Conflict resolution supports accept current, accept incoming, accept both/combination, ignore, and direct result editing.
- Conflict count remains visible.
- Alternative layouts and base view are available.
- AI conflict resolution exists, but is explicitly experimental and subscription-dependent.

**Lesson for Forskscope:** adopt the UX pattern of "explicit choices plus editable result", but avoid becoming a Git commit/staging client.

Candidate default features:

- Three-pane merge layout: Left/Right/Result or Incoming/Current/Result.
- Clear hunk-level actions: accept left, accept right, accept both, ignore, edit result.
- Conflict count and unresolved conflict navigator.
- Direct editing of the result buffer.
- Optional base panel in three-way merge.
- Side-by-side and inline diff layout modes.

Candidate optional features:

- VCS-aware mode that can read Git status and show file changes.
- Partial patch export from selected hunks/ranges.
- AI-assisted conflict explanation, disabled by default.

Non-goals:

- Commit authoring.
- Branch graph.
- Pull/push/fetch.
- PR review.
- Full source-control dashboard.

---

### 2.3 IntelliJ IDEA diff and merge views

IntelliJ IDEA provides strong lessons for editor integration and user confidence:

- compare two or three files
- compare active editor with clipboard
- compare active editor with project/external file
- folder comparison by size/content/timestamp
- diff viewer in a tab
- powerful editor-backed diff view
- automatic application of edits from the diff viewer to files
- three-pane conflict resolution with read-only sides and editable center result
- automatic merge of non-conflicting changes
- gutter highlighting for modified lines
- next-file navigation after last change

**Lesson for Forskscope:** do not hide the result model. Make merge result an explicit, editable, safe buffer. Also support quick entry points such as clipboard compare.

Candidate default features:

- Compare two files.
- Compare three files.
- Compare clipboard vs file / clipboard vs clipboard.
- Compare active result buffer with another file.
- Editable result buffer with clear save target.
- Auto-apply non-conflicting changes, but only with visible audit and undo.
- Next/previous change and next/previous file navigation.
- Folder comparison by size, timestamp, and content/hash modes.

Candidate optional features:

- Syntax-aware editor services.
- Project-aware comparison when launched inside a repository or directory tree.
- External editor integration.

Non-goals:

- Full IDE functionality.
- Code completion as a primary product feature.
- Refactoring engine.

---

### 2.4 Meld

Meld is closest to Forskscope's Linux/Unix worker identity:

- file, directory, and version-controlled project comparison
- two- and three-way comparison of files and directories
- live comparison updates
- change-block navigation
- simple text filtering to ignore irrelevant differences
- three-way merge assistance with conflict handling and base version display

**Lesson for Forskscope:** keep the product focused and approachable. Meld proves that a local visual diff/merge tool can remain useful without becoming an IDE.

Candidate default features:

- Live diff refresh after editing.
- Change-block navigation.
- Text filters for ignoring irrelevant differences.
- File/directory/project-oriented start screen.
- Three-way merge with base display.

Candidate optional features:

- VCS project mode with shallow repository awareness.
- Saved filters per workspace.

Non-goal:

- Heavy IDE or source-control management behavior.

---

### 2.5 KDiff3

KDiff3 provides especially valuable lessons for directory merge and safety:

- recursive comparison/merge of two or three folders
- symbolic-link handling
- double-click browse into files
- proposed merge operation per item
- ability to change proposed operations before execution
- simulation/dry-run before folder merge
- warning that folder merge can affect many files and needs backups
- line-by-line and character-by-character differences
- automatic merge facility
- integrated editor for merge conflict resolution
- Unicode/UTF-8/BOM and codec considerations
- manual line alignment

**Lesson for Forskscope:** folder merge must be treated as a planned operation with preview, simulation, backup, and rollback semantics. It must not be a set of immediate destructive actions.

Candidate default features:

- Directory merge plan view.
- Proposed operation column: copy left to right, copy right to left, delete, keep, conflict, skip.
- Dry-run/simulation before execution.
- Backup requirement or safety confirmation before destructive directory merge.
- Symlink policy.
- Manual line alignment.
- Encoding/BOM detection display.

Candidate optional features:

- Three-way folder merge.
- Batch operation queue with pause/resume.
- Recursive policy templates.

Non-goal:

- Silent automatic folder synchronization.

---

### 2.6 Beyond Compare

Beyond Compare provides lessons for professional-grade comparison workflows:

- file/folder merge
- separate output location or overwrite input
- automatic merge of text files
- background operations
- pause running operations
- printed/HTML comparison reports
- linked folder reports to file reports
- criteria-based folder comparison: timestamps, sizes, attributes, contents
- filename alignment with wildcard masks
- Unicode normalization alignment across platforms
- optional remote/cloud/FTP/SFTP support

**Lesson for Forskscope:** professional users need sessions, profiles, reports, and safe destination control. Remote/cloud support is attractive but should not enter the core.

Candidate default features:

- Named comparison profiles.
- Folder compare criteria: timestamp, size, content/hash, attributes where available.
- HTML/text report export.
- Separate merge output directory.
- Unicode normalization policy for cross-platform folder comparisons.

Candidate optional features:

- Background operation queue with pause/cancel.
- Advanced filename alignment rules.
- Remote filesystem providers as plugins only.

Non-goals:

- FTP/SFTP/cloud storage in the core app.
- General file manager replacement.
- Enterprise sync product.

---

## 3. Recommended Default Feature Candidates

These should be treated as baseline product-quality features for a serious Forskscope v1.x line.

| ID | Candidate | Source inspiration | Why it belongs by default | Related existing RFC |
|---|---|---|---|---|
| D-01 | Explicit result buffer for merge | VS Code, IntelliJ, KDiff3 | Prevents hidden mutation and supports manual merge | RFC-021, RFC-032, RFC-034 |
| D-02 | Hunk actions: accept left/right/both/ignore/edit | VS Code, Visual Studio, IntelliJ | Core merge ergonomics | RFC-006, RFC-034 |
| D-03 | Conflict count and unresolved navigator | VS Code | Users must know when merge is complete | RFC-034, RFC-035 |
| D-04 | Optional base view in 3-way merge | VS Code, IntelliJ, Meld | Critical for conflict understanding | RFC-033, RFC-034 |
| D-05 | Auto-merge non-conflicting changes with audit | IntelliJ, Beyond Compare | Saves effort while keeping trust | RFC-033, RFC-034 |
| D-06 | Side-by-side and inline diff views | VS Code, Git | Different tasks need different layouts | RFC-024, RFC-035 |
| D-07 | Context lines and collapse unchanged fragments | IntelliJ, Git | Essential for large diffs | RFC-013, RFC-024 |
| D-08 | Ignore whitespace / trim / line-ending options | Git, VS Code | Reduces noise in cross-platform files | RFC-012, RFC-028 |
| D-09 | Moved-line or moved-block detection | Git | Helps review refactors and reorderings | New RFC candidate |
| D-10 | Change-block navigation across files | Meld, IntelliJ | WinMerge-class review flow | RFC-014, RFC-035 |
| D-11 | Compare clipboard with file/text | IntelliJ | Very useful quick task, low architectural risk | New RFC candidate |
| D-12 | Directory merge plan with operation column | KDiff3 | Makes folder merge understandable and safe | RFC-022, RFC-037 |
| D-13 | Directory merge dry-run | KDiff3 | Prevents destructive surprises | RFC-022, RFC-023 |
| D-14 | Backup/restore for destructive actions | KDiff3, Beyond Compare | Required for trust | RFC-023 |
| D-15 | Named comparison profiles | Beyond Compare, Git config | Keeps advanced options manageable | RFC-028 |
| D-16 | HTML/text report export | Beyond Compare, KDiff3 | Useful for audits and collaboration | RFC-027 |
| D-17 | Unicode normalization/BOM/encoding visibility | KDiff3, Beyond Compare | Important for cross-platform users | RFC-012 |
| D-18 | Manual line alignment | KDiff3 | Solves cases where automatic diff alignment is poor | New RFC candidate |
| D-19 | Open as external diff/merge tool from CLI | Git, IntelliJ, KDiff3 | Enables use as `git difftool` / `git mergetool` | RFC-029, RFC-038 |
| D-20 | Simpler VCS context indicator | VS Code, Meld | Helpful without becoming a Git client | RFC-038 |

---

## 4. Recommended Optional Feature Candidates

These are useful, but should be hidden behind settings, profiles, plugins, or post-v1 RFCs.

| ID | Candidate | Why optional |
|---|---|---|
| O-01 | Diff algorithm selector | Powerful, but confusing for normal users |
| O-02 | Patience/histogram-like algorithms | Useful for source code, but implementation/testing cost is non-trivial |
| O-03 | Partial patch export from selected hunks/ranges | Useful for Git workflows, but not a baseline local merge need |
| O-04 | Lightweight Git worktree status panel | Useful, but risks becoming a Git client |
| O-05 | Background batch operation queue with pause/resume | Valuable for large directory merges, but post-core hardening |
| O-06 | Advanced filename alignment rules | Useful for generated files, but niche |
| O-07 | Remote filesystem providers | Strong feature, high security and complexity risk |
| O-08 | Syntax-aware diff decorations | Useful for code, but language-specific |
| O-09 | Refactoring-aware diff | Valuable but research/product complexity is high |
| O-10 | AI conflict explanation/resolution | Must be local/optional/explicit due to trust, privacy, and correctness concerns |
| O-11 | Image/PDF/table/domain-specific comparison | Useful, but should be separate modules rather than core text merge |
| O-12 | Compare with local history snapshots | Useful but requires a local history subsystem |
| O-13 | Gutter diff indicators in ordinary editor mode | Useful if Forskscope becomes an editor, not required for standalone compare sessions |
| O-14 | Report bundle linking folder report to file reports | Good professional feature, but can follow simple report export |
| O-15 | Full command palette | Useful for power users, already planned, but should not overload v1 UI |

---

## 5. Explicit Non-Goals

To protect product focus, these should be declared as non-goals unless the business strategy changes.

| ID | Non-goal | Reason |
|---|---|---|
| N-01 | Full Git client | Forskscope should complement Git, not replace Git GUIs |
| N-02 | Commit authoring and branch graph | Too IDE/source-control oriented |
| N-03 | Pull request review platform | Requires hosting integration and collaboration model |
| N-04 | Cloud sync product | Security, credentials, and support burden |
| N-05 | Remote FTP/SFTP/WebDAV in core | Useful but too much attack surface for v1 |
| N-06 | Code completion/refactoring engine | IDE scope creep |
| N-07 | AI auto-resolution by default | Trust and correctness risk |
| N-08 | Silent folder synchronization | Dangerous for user data |
| N-09 | Database/object compare | Outside current product identity |
| N-10 | Binary reverse-engineering viewer | Outside text/directory merge baseline |

---

## 6. Proposed New RFC Addendum

The previous RFC set remains valid. The competitive analysis suggests the following additional RFCs or amendments.

### RFC-042: Competitive Feature Adoption Policy

Purpose:

- Define how features are adopted from Git, IDEs, and commercial diff tools.
- Prevent scope creep into Git-client, IDE, or sync-product territory.
- Establish default/optional/non-goal classification criteria.

Key decisions:

- A feature is default only if it directly improves local compare/merge safety, clarity, or efficiency.
- VCS features are context providers, not workflow owners.
- AI features are optional, explicit, and never required for correctness.

---

### RFC-043: Advanced Diff Readability Options

Purpose:

- Introduce moved-block highlighting, whitespace error highlighting, context folding, and optional algorithm profiles.

Default scope:

- Ignore whitespace modes.
- Context lines.
- Collapse unchanged blocks.
- Moved-block hinting when safe.
- Whitespace error visualization.

Optional scope:

- Algorithm selector.
- Expert diff profiles.

---

### RFC-044: Clipboard and Ad-Hoc Compare Workflows

Purpose:

- Add quick comparison workflows inspired by IntelliJ IDEA.

Default scope:

- Compare clipboard with file.
- Compare clipboard with editor/result buffer.
- Compare two pasted text buffers.
- Create temporary unsaved compare sessions.

Non-goals:

- Persistent clipboard history.
- Sensitive clipboard telemetry.

---

### RFC-045: Manual Alignment and Diff Correction Workflow

Purpose:

- Support cases where automatic diff alignment is visually wrong.

Default scope:

- Mark line as alignment anchor.
- Align selected left/right lines.
- Clear manual alignment.
- Show manual-alignment indicator.

Risks:

- Manual alignment must affect view/diff session only unless explicitly saved as a session note.
- Must not silently mutate file content.

---

### RFC-046: Directory Merge Plan, Simulation, and Operation Review

Purpose:

- Strengthen RFC-022/RFC-023/RFC-037 with KDiff3/Beyond Compare-style folder merge planning.

Default scope:

- Operation column.
- Status/statistics columns.
- Dry-run simulation.
- Backup/restore integration.
- Separate output destination option.
- Symlink and Unicode normalization policy display.

Non-goal:

- Silent sync.

---

### RFC-047: VCS Tool Integration Without Becoming a Git Client

Purpose:

- Support `git difftool`, `git mergetool`, and shallow VCS context.

Default scope:

- CLI invocation forms for two-way and three-way compare.
- Respect Git mergetool file role conventions.
- Show repository context if detected.
- Patch export/apply compatibility.

Optional scope:

- Read Git status for file grouping.
- Show blame or timeline only through external command integration.

Non-goals:

- Commit UI.
- Branch graph.
- Pull/push/fetch.

---

### RFC-048: Review-Oriented Report Bundles

Purpose:

- Extend report export into professional review evidence.

Default scope:

- Single file comparison report.
- Directory comparison summary report.
- Export selected hunks.
- Include settings/profile used for comparison.

Optional scope:

- Linked folder report to individual file reports.
- Redaction rules.

---

### RFC-049: Optional AI Assistance Boundary

Purpose:

- Define a safe boundary before any AI conflict feature is considered.

Default scope:

- No AI dependency.
- No automatic AI writes.
- No remote transmission unless user explicitly configures it.
- AI may explain conflicts or propose patch candidates only in optional mode.

Non-goal:

- Default auto-merge by AI.

---

## 7. Priority Recommendation

### Immediate amendments to existing RFCs

1. Add clipboard/ad-hoc compare to explorer/workspace scope.
2. Add manual alignment to diff visual semantics and editor adapter scope.
3. Add moved-block and whitespace error highlighting to diff readability scope.
4. Strengthen directory merge plan/dry-run behavior.
5. Clarify Git integration boundary.

### New RFCs to create next

Recommended order:

1. RFC-042 Competitive Feature Adoption Policy
2. RFC-043 Advanced Diff Readability Options
3. RFC-044 Clipboard and Ad-Hoc Compare Workflows
4. RFC-045 Manual Alignment and Diff Correction Workflow
5. RFC-046 Directory Merge Plan, Simulation, and Operation Review
6. RFC-047 VCS Tool Integration Without Becoming a Git Client
7. RFC-048 Review-Oriented Report Bundles
8. RFC-049 Optional AI Assistance Boundary

### Highest-value default features

If we can only add a few candidates, prioritize:

1. Three-pane result editor with accept/ignore/edit actions.
2. Conflict count and unresolved navigator.
3. Clipboard compare.
4. Context folding and whitespace modes.
5. Directory merge plan with dry-run.
6. Separate output destination and backup/restore.
7. Git difftool/mergetool CLI compatibility.
8. Manual alignment.
9. Moved-block detection.
10. HTML/text report export.

---

## 8. Source References

Accessed: 2026-06-08

- Git `git-diff` documentation: https://git-scm.com/docs/git-diff
- VS Code Source Control overview: https://code.visualstudio.com/docs/sourcecontrol/overview
- VS Code staging and committing changes: https://code.visualstudio.com/docs/sourcecontrol/staging-commits
- VS Code merge conflicts: https://code.visualstudio.com/docs/sourcecontrol/merge-conflicts
- IntelliJ IDEA compare files, folders, and text sources: https://www.jetbrains.com/help/idea/comparing-files-and-folders.html
- IntelliJ IDEA Diff Viewer for files: https://www.jetbrains.com/help/idea/differences-viewer.html
- IntelliJ IDEA resolve Git conflicts: https://www.jetbrains.com/help/idea/resolve-conflicts.html
- IntelliJ IDEA Diff & Merge settings: https://www.jetbrains.com/help/idea/settings-tools-diff-and-merge.html
- Meld visual diff and merge tool: https://gnome.pages.gitlab.gnome.org/meld/
- KDiff3 folder comparison and merge documentation: https://docs.kde.org/stable_kf6/en/kdiff3/kdiff3/dirmerge.html
- KDiff3 legacy feature overview: https://kdiff3.sourceforge.net/
- Beyond Compare feature comparison: https://www.scootersoftware.com/kb/feature_compare
- Beyond Compare folder compare help: https://www.scootersoftware.com/v5help/dir_how_to_compare.html
- Visual Studio merge conflict resolution: https://learn.microsoft.com/en-us/visualstudio/version-control/git-resolve-conflicts
