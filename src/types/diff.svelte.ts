import type { DIFF_KIND } from "../consts"

export type DiffKind = typeof DIFF_KIND[number]

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
