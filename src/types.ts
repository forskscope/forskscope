export interface DiffResponse {
    oldCharset: string
    newCharset: string
    linesDiffs: LinesDiff[]
}

export interface LinesDiff {
    diffKind: DiffKind
    linesCount: number
    oldLines: string[]
    newLines: string[]
    replaceDetail: ReplaceDetailLinesDiff | null
}

export interface ReplaceDetailLinesDiff {
    oldLines: ReplaceDiffChars[][]
    newLines: ReplaceDiffChars[][]
}

export interface ReplaceDiffChars {
    diffKind: DiffKind
    chars: string
}

export type OldOrNew = "old" | "new"

export type DiffKind = "equal" | "delete" | "insert" | "replace"

export const APP_THEMES = ['light-theme', 'dark-theme', 'night-theme', 'monokai-theme']
export type AppTheme = typeof APP_THEMES[number]

export interface DiffFilepaths {
    old: string,
    new: string,
}

export interface ListDirReponse {
    currentDir: string,
    dirs: string[],
    files: string[],
}
