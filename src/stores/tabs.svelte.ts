import { writable, type Writable } from "svelte/store";
import type { CompareSet } from "../types";

export const EXPLORER_TAB_INDEX: number = 0 // todo: -1

export const compareSets: Writable<CompareSet[]> = writable([])

export const activeTabIndex: Writable<number> = writable(EXPLORER_TAB_INDEX)
