import { get, writable, type Writable } from "svelte/store";
import type { CompareSet } from "../types/compareSets.svelte";
import { binaryComparisonOnly } from "../utils/compareSets.svelte";

let activeCompareSetIndex: number | null = $state(null)

export const compareSets: Writable<CompareSet[]> = writable([])

export const pushCompareSet = (compareSet: CompareSet) => {
    compareSets.update((x) => { x.push(compareSet); return x })
    activeCompareSetIndex = get(compareSets).length - 1
}

export const spliceCompareSet = (index: number) => {
    compareSets.update((x) => { x.splice(index, 1); return x })

    if (get(compareSets).length === 0) {
        activeCompareSetIndex = null
        return
    }

    const activeCompareSetIndexIsPreserved =
        activeCompareSetIndex === null ||
        activeCompareSetIndex === 0 ||
        activeCompareSetIndex < index
    if (!activeCompareSetIndexIsPreserved) {
        activeCompareSetIndex! -= 1
    }
}

export const getCompareSet = (index: number): CompareSet => {
    return get(compareSets)[index]
}

export const updateCompareSet = async (index: number, oldFilepath: string, newFilepath: string): Promise<CompareSet> => {
    const oldBinaryComparisonOnly = await binaryComparisonOnly(oldFilepath)
    const newBinaryComparisonOnly = await binaryComparisonOnly(newFilepath)

    const _compareSet = getCompareSet(index)!
    _compareSet.old.filepath = oldFilepath
    _compareSet.old.binaryComparisonOnly = oldBinaryComparisonOnly
    _compareSet.new.filepath = newFilepath
    _compareSet.new.binaryComparisonOnly = newBinaryComparisonOnly

    return _compareSet
}

export const isActiveCompareSetIndex = (index: number | null): boolean => {
    return index === activeCompareSetIndex
}

export const activateCompareSet = (index: number) => {
    activeCompareSetIndex = index
}

export const removeCompareSet = (index: number) => {
    spliceCompareSet(index)

    if (activeCompareSetIndex !== null) {
        // ask svelte to update view
        const i = activeCompareSetIndex
        activeCompareSetIndex = null
        activeCompareSetIndex = i
    }
}

export const activateExplorer = () => {
    activeCompareSetIndex = null
}

export const isActiveCompareSet = (index: number | null) => {
    return index !== null && index === activeCompareSetIndex
}

export const exploreIsActive = (): boolean => {
    return activeCompareSetIndex === null
}