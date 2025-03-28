import { get, writable, type Writable } from "svelte/store";
import type { CompareSet } from "../types";

let activeCompareSetIndex: number | null = $state(null)

export const compareSets: Writable<CompareSet[]> = writable([])

export const activeCompareSet = (): CompareSet | null => {
    if (activeCompareSetIndex === null) return null
    return get(compareSets)[activeCompareSetIndex!]
}

export const pushCompareSet = (compareSet: CompareSet) => {
    compareSets.update((x) => { x.push(compareSet); return x })
    activeCompareSetIndex = get(compareSets).length - 1
}

export const spliceCompareSet = (index: number) => {
    compareSets.update((x) => { x.splice(index, 1); return x })
    if (get(compareSets).length === 0) {
        activeCompareSetIndex = null
    }
    else if (activeCompareSetIndex === index) {
        activeCompareSetIndex = activeCompareSetIndex! - 1
    }
}

export const activateCompareSet = (index: number) => {
    activeCompareSetIndex = index
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