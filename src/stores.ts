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
export function removeDiffTabsStore(index: number) {
  const u1 = diffTabIndexStore.subscribe(diffTabIndex => {
    const u2 = diffTabsStore.subscribe(diffTabs => {
      if (diffTabIndex === 0) return
      const isLastTabActive = diffTabIndex === diffTabs.length - 1
      if (isLastTabActive) {
        diffTabIndexStore.update(current => current - 1)
      }
    })
    u2()
  })
  u1()
  
  diffTabsStore.update(current => {
    return current.filter((_, i) => i !== index)
  })
}

// selected index on diff tabs
export const diffTabIndexStore: Writable<number> = writable(0)
export function updateDiffTabIndexStore(value: number) {
  diffTabIndexStore.update(_ => value)
}
