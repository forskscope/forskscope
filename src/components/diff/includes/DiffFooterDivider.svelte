<script lang="ts">
  import { ArrowRightLeft, Menu, RefreshCw, ScanSearch, Undo } from 'lucide-svelte'
  import { T } from '../../../stores/settings/translation.svelte'
  import type { LinesDiffResponse, MergeHistoryItem } from '../../../types/diff'

  const {
    mergeHistory,
    toggleCharsDiffs,
    switchFilepaths,
    undoMergeHistory,
    reloadDiff,
  }: {
    mergeHistory: MergeHistoryItem[]
    toggleCharsDiffs: () => void
    switchFilepaths: () => void
    undoMergeHistory: () => void
    reloadDiff: () => void
  } = $props()

  let showsMenus: boolean = $state(false)

  const undoMergeHistoryEnabled: boolean = $derived(0 < mergeHistory.length)
</script>

<div class="menus-wrapper">
  <button class={`toggle ${showsMenus ? 'opened' : ''}`} onclick={() => (showsMenus = !showsMenus)}
    ><Menu /></button
  >

  <div class={`menus ${showsMenus ? 'active' : 'd-none'}`}>
    <button onclick={toggleCharsDiffs}>
      <ScanSearch />
      {T('Show chars diff')}
    </button>
    <button onclick={switchFilepaths}>
      <ArrowRightLeft />
      {T('Switch left/right')}
    </button>
    <button onclick={undoMergeHistory} disabled={!undoMergeHistoryEnabled}>
      <Undo />
      {T('Undo a merge history')}
    </button>
    <button onclick={reloadDiff}>
      <RefreshCw />
      {T('Reload diff')}
    </button>
  </div>
</div>

<style>
  .menus-wrapper {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .menus {
    position: absolute;
    bottom: 2.2rem;
    width: fit-content;
    height: fit-content;
    display: flex;
    gap: 0.3rem;
    opacity: 0.87;
  }

  .toggle {
    padding: 0.2rem;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    transition: transform 0.3s ease-out;
  }

  .menus-wrapper:has(.menus.active) .toggle {
    transform: rotate(90deg);
  }
</style>
