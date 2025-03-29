import { invoke } from "@tauri-apps/api/core"
import { createCompareSetItem, type CompareSet, type CompareSetItem } from "../types"
import { pushCompareSet } from "../stores/compareSets.svelte"

export const binaryComparisonOnly = async (filepath: string): Promise<boolean> => {
    const res = (await invoke('binary_comparison_only', { filepath })
        // todo
        .catch((error: unknown) => {
            console.error(error)
            return
        }))
    return res as unknown as boolean
}

export const filepathsToCompareSet = async (filepaths: string[] | null) => {
    if (filepaths === null || filepaths.length === 0) return

    const oldFilepath = filepaths[0]
    const oldBinaryComparisonOnly = await binaryComparisonOnly(oldFilepath)
    const oldCompareSetItem = {
        filepath: oldFilepath,
        binaryComparisonOnly: oldBinaryComparisonOnly,
    } as CompareSetItem

    let newCompareSetItem: CompareSetItem = createCompareSetItem()
    if (1 < filepaths.length) {
        const newFilepath = filepaths[1]
        newCompareSetItem.filepath = newFilepath
        newCompareSetItem.binaryComparisonOnly = await binaryComparisonOnly(newFilepath)
    }

    const compareSet = { old: oldCompareSetItem, new: newCompareSetItem } as CompareSet
    pushCompareSet(compareSet)
}
