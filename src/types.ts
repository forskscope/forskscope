export interface LinesDiff {
    diffKind: string
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
    diffKind: string
    chars: string
}

export type OldOrNew = "old" | "new"
