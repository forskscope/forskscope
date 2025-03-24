export interface DiffResponse {
    oldCharset: string
    newCharset: string
    linesDiffs: LinesDiff[]
}

export interface LinesDiff {
    diffIndex: number
    diffKind: DiffKind
    linesCount: number
    oldLines: string[]
    newLines: string[]
}

export interface CharsDiffResponse {
    diffs: CharsDiffLines[],
}

export interface CharsDiffLines {
    diffIndex: number,
    oldLines: CharsDiff[][]
    newLines: CharsDiff[][]
}

export interface CharsDiff {
    diffKind: DiffKind
    chars: string
}

export type OldOrNew = "old" | "new"

export type DiffKind = "equal" | "delete" | "insert" | "replace"

export const APP_THEMES = ['light-theme', 'dark-theme', 'night-theme', 'monokai-theme']
export type AppTheme = typeof APP_THEMES[number]

export const APP_DIFF_FONT_FAMILIES = ['sans-serif-diff-font-family', 'serif-diff-font-family', 'monospace-diff-font-family']
export type AppDiffFontFamily = typeof APP_DIFF_FONT_FAMILIES[number]

export const APP_UI_FONT_FAMILIES = ['sans-serif-ui-font-family', 'serif-ui-font-family', 'monospace-ui-font-family']
export type AppUiFontFamily = typeof APP_UI_FONT_FAMILIES[number]

export interface DiffFilepaths {
    old: string,
    new: string,
}

export interface StartupParam {
    oldFilepath: string | null,
    newFilepath: string | null,
}

export interface ListDirReponse {
    currentDir: string,
    dirs: string[],
    files: FileAttr[],
}

export interface FileAttr {
    name: string,
    bytesSize: string,
    humanReadableSize: string,
    lastModified: string,
}

export const APP_LANGUAGES = ['en', 'ja']
export type AppLanguage = typeof APP_LANGUAGES[number]
