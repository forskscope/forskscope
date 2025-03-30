import type { OLD_OR_NEW } from "../consts"

export type OldOrNew = typeof OLD_OR_NEW[number]

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
