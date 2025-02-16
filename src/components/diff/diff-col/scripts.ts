import type { LinesDiff, OldOrNew, ReplaceDetailLinesDiff, ReplaceDiffChars } from "../../../types"

export const diffLines = (linesDiff: LinesDiff, oldOrNew: OldOrNew): string[] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
}

export const replaceDetailLines = (linesDiff: ReplaceDetailLinesDiff, oldOrNew: OldOrNew): ReplaceDiffChars[][] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
}
