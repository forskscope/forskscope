import { writable, type Writable } from 'svelte/store'
import { type DiffTab } from './types'

// diff tabs
export const diffTabsStore: Writable<DiffTab[]> = writable([])
export function pushToDiffTabsStore(value: DiffTab) {
  diffTabsStore.update(current => {
    const updated = [...current, value]
    updateDiffTabIndexStore(updated.length - 1)
    return updated
  })
}

// selected index on diff tabs
export const diffTabIndexStore: Writable<number> = writable(0)
export function updateDiffTabIndexStore(value: number) {
  diffTabIndexStore.update(_ => value)
}
