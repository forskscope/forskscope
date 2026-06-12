# RFC-009 — Settings, Theme, Localization, and Accessibility

**Status.** Implemented (v0.60.0) — core complete; settings dialog UI, LocaleBundle, i18n message IDs deferred to UI layer

---toml
project = "ForskScope"
rfc = "009"
title = "Settings, Theme, Localization, and Accessibility"
status = "proposed"
phase = "M9"
depends_on = ["RFC-003"]
---

## 1. Summary

Define user settings, theme model, typography, localization, keyboard accessibility, and diff color semantics for the Dioxus migration. The goal is to preserve the current app's theme/localization intent while making settings more explicit and accessible.

## 2. Goals

- Provide a settings dialog with clear categories.
- Preserve themes: light/dark/night or equivalent.
- Support typography configuration useful for code/text diff work.
- Support localization, initially English and Japanese if resources exist.
- Ensure diff status is not represented by color alone.
- Define keyboard and screen-reader requirements.

## 3. Non-Goals

- Build a full plugin/theme marketplace.
- Implement arbitrary user CSS injection.
- Translate all documentation in this RFC.
- Guarantee WCAG audit certification in first release.

## 4. Settings Model

```rust
pub struct UserSettings {
    pub appearance: AppearanceSettings,
    pub diff: DiffSettings,
    pub editor: EditorSettings,
    pub files: FileSettings,
    pub shortcuts: ShortcutSettings,
    pub locale: LocaleSettings,
}

pub struct AppearanceSettings {
    pub theme: ThemeId,
    pub density: Density,
    pub font_family: FontFamilySetting,
    pub font_size: u8,
}
```

## 5. Settings Dialog Wireframe

```text
┌──────────────────────────────────────────────────────────────┐
│ Settings                                                     │
├───────────────┬──────────────────────────────────────────────┤
│ Appearance    │ Theme: [Dark ▼]                              │
│ Diff          │ Density: [Comfortable ▼]                     │
│ Editor        │ Font: [System Mono ▼] Size: [14]             │
│ Files         │                                              │
│ Shortcuts     │ Preview:                                     │
│ Language      │  - removed line                              │
│ Diagnostics   │  + added line                                │
├───────────────┴──────────────────────────────────────────────┤
│ [Reset Category]                         [Cancel] [Apply]    │
└──────────────────────────────────────────────────────────────┘
```

## 6. Theme Semantics

Theme tokens should be semantic:

```rust
pub struct ThemeTokens {
    pub app_bg: ColorToken,
    pub panel_bg: ColorToken,
    pub text_primary: ColorToken,
    pub text_muted: ColorToken,
    pub border_subtle: ColorToken,
    pub diff_equal_bg: ColorToken,
    pub diff_insert_bg: ColorToken,
    pub diff_delete_bg: ColorToken,
    pub diff_replace_bg: ColorToken,
    pub focus_ring: ColorToken,
    pub warning: ColorToken,
    pub error: ColorToken,
}
```

CSS variables may be generated from theme tokens in the Dioxus app.

## 7. Diff Accessibility Rules

Diff meaning must be represented by:

- Color.
- Symbol or label.
- Screen-reader text.
- Optional gutter marker.

Examples:

```text
+ inserted
- deleted
~ replaced
= equal
! conflict/error
```

## 8. Keyboard Requirements

- All toolbar commands are keyboard reachable.
- Tab switching is keyboard reachable.
- Explorer rows are navigable without mouse.
- Hunk navigation shortcuts are documented.
- Modal focus is trapped until closed.
- Escape closes non-destructive dialogs.

## 9. Localization Model

```rust
pub struct LocaleBundle {
    pub locale: LocaleId,
    pub messages: HashMap<MessageId, String>,
}
```

Message IDs must be stable and not based on English prose.

Example:

```text
app.command.open_files
app.command.save
explorer.state.left_only
diff.hunk.replace
save.conflict.external_modified
```

## 10. Persistence

Settings should be stored in a user config file using a stable format such as TOML or JSON.

Rules:

- Unknown fields are ignored for forward compatibility.
- Missing fields receive defaults.
- Corrupt settings file causes a warning and fallback to defaults.
- The diagnostics panel shows settings file path.

## 11. User Workflows

### 11.1 Change Theme

```text
Open Settings
→ Appearance
→ choose theme
→ preview applies immediately
→ Apply saves setting
```

### 11.2 Change Diff Font

```text
Open Settings
→ Editor
→ choose monospace font and size
→ preview updates diff sample
→ Apply saves setting
```

### 11.3 Change Language

```text
Open Settings
→ Language
→ choose locale
→ app updates labels
→ restart not required if feasible
```

## 12. Testing Requirements

- Settings defaults load with no file.
- Corrupt settings file does not crash app.
- Theme tokens apply to diff rows.
- Keyboard navigation reaches all primary controls.
- Modal focus trap works.
- Japanese locale can render without layout breakage.
- High-contrast theme or equivalent passes basic contrast checks.

## 13. Acceptance Criteria

- Settings dialog exists with Appearance, Diff, Editor, Files, Shortcuts, Language, Diagnostics categories.
- Theme tokens drive app and diff colors.
- Diff semantics do not rely on color alone.
- Keyboard navigation and focus handling are documented and tested.
- Localization system supports at least English and can host Japanese messages.

## 14. Risks

| Risk | Mitigation |
|---|---|
| Visual design becomes too web-like | Use desktop workstation layout and dense controls. |
| User CSS/customization breaks UI | Prefer controlled theme tokens. |
| Localization strings drift | Use stable message IDs and missing-string tests. |
| Accessibility is postponed | Include in acceptance criteria for every UI RFC. |
