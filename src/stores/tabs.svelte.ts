import { get, writable, type Writable } from "svelte/store";
import type { CompareSet } from "../types";

export const EXPLORER_TAB_INDEX: number = 0 // todo: -1

export const compareSets: Writable<CompareSet[]> = writable([])

export const activeTabIndex: Writable<number> = writable(EXPLORER_TAB_INDEX)

export const pushCompareSet = (compareSet: CompareSet) => {
    compareSets.update((x) => { x.push(compareSet); return x })
    activeTabIndex.set(get(compareSets).length)
}

export const spliceCompareSet = (index: number) => {
    if (get(activeTabIndex) === index) {
        activeTabIndex.set(get(activeTabIndex) - 1)
    }
    compareSets.update((x) => { x.splice(index, 1); return x })
}
