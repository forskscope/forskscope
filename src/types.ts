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