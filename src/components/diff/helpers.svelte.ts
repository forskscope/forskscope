import type { BackendCommandResult } from "../../types/backend"
import type { CharsDiffLines, CharsDiffResponse, LinesDiff, LinesDiffResponse } from "../../types/diff"
import { invokeWithGuard } from "../../utils/backend.svelte"
import { DIFF_MAIN_CLASS_PREFIX } from "./consts"

export const diffCharsFromLinesDiffResponse = async (linesDiffResponse: LinesDiffResponse): Promise<CharsDiffResponse | null> => {
    const replaceLinesDiffs = linesDiffResponse.diffs.filter((x) => x.diffKind === 'replace')
    if (replaceLinesDiffs.length === 0) return null

    const res = await invokeWithGuard('diff_chars', {
        linesDiffs: replaceLinesDiffs,
    })
    if (res.isError) return null
    return res.response as unknown as CharsDiffResponse
}

export const deltaModifyingFocusedDiffLinesIndex = (
    delta: number,
    focusedLinesDiffIndex: number | null,
    linesDiffResponse: LinesDiffResponse | null,
): number | null => {
    if (!linesDiffResponse) return null

    const _firstFocusedLinesDiffIndex = firstFocusedLinesDiffIndex(linesDiffResponse)

    if (focusedLinesDiffIndex === null ||
        (focusedLinesDiffIndex === _firstFocusedLinesDiffIndex && delta < 0)) {
        return _firstFocusedLinesDiffIndex
    }

    const _lastFocusedLinesDiffIndex = lastFocusedLinesDiffIndex(linesDiffResponse)

    if (focusedLinesDiffIndex === _lastFocusedLinesDiffIndex && 0 < delta) {
        return _lastFocusedLinesDiffIndex
    }

    if (0 < delta) {
        return linesDiffResponse.diffs.findIndex((x, i) => {
            return x.diffKind !== 'equal' && focusedLinesDiffIndex! < i
        })
    } else {
        return linesDiffResponse.diffs.findLastIndex((x, i) => {
            return x.diffKind !== 'equal' && i < focusedLinesDiffIndex!
        })
    }
}

export const scrollIntoFocusedDiffLines = (compareSetIndex: number) => {
    document
        .querySelector(`.${DIFF_MAIN_CLASS_PREFIX}${compareSetIndex} .content .new .focused`)!
        .scrollIntoView({ behavior: 'smooth', block: 'center', inline: 'start' })
}

export const switchLinesDiffs = (linesDiffs: LinesDiff[]): LinesDiff[] => {
    return linesDiffs.map((x) => {
        const ret = x
        const orgOldLines = ret.oldLines
        ret.oldLines = ret.newLines
        ret.newLines = orgOldLines
        return ret
    })
}

export const switchCharsDiffLinesList = (charsDiffs: CharsDiffLines[]): CharsDiffLines[] => {
    return charsDiffs.map((x) => {
        const ret = x
        const orgOldLines = ret.oldLines
        ret.oldLines = ret.newLines
        ret.newLines = orgOldLines
        return ret
    })
}

const firstFocusedLinesDiffIndex = (linesDiffResponse: LinesDiffResponse | null): number => {
    if (!linesDiffResponse) return -1
    return linesDiffResponse.diffs.findIndex((x) => x.diffKind !== 'equal')
}

const lastFocusedLinesDiffIndex = (linesDiffResponse: LinesDiffResponse | null): number => {
    if (!linesDiffResponse) return -1
    return linesDiffResponse.diffs.findLastIndex((x) => x.diffKind !== 'equal')
}