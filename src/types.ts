import type { APP_DIFF_FONT_FAMILIES, APP_LANGUAGES, APP_THEMES, APP_UI_FONT_FAMILIES, DIFF_KIND, OLD_OR_NEW } from "./consts"

export type AppLanguage = typeof APP_LANGUAGES[number]

export type AppTheme = typeof APP_THEMES[number]
export type AppDiffFontFamily = typeof APP_DIFF_FONT_FAMILIES[number]
export type AppUiFontFamily = typeof APP_UI_FONT_FAMILIES[number]

export type DiffKind = typeof DIFF_KIND[number]

export type OldOrNew = typeof OLD_OR_NEW[number]

export interface LinesDiffResponse {
    oldCharset: string
    newCharset: string
    diffs: LinesDiff[]
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

export interface CompareSet {
    old: CompareSetItem,
    new: CompareSetItem,
}

export interface CompareSetItem {
    filepath: string,
    binaryComparisonOnly: boolean,
}

export function createCompareSetItem(): CompareSetItem {
    return {
        filepath: "",
        binaryComparisonOnly: false
    } as CompareSetItem
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
    binaryComparisonOnly: boolean
}

export interface BackendCommandResult {
    response: unknown
    isError: boolean
}
