<script lang="ts">
  import { getCurrent, type DragDropEvent } from "@tauri-apps/api/webview"
  import type { UnlistenFn, Event as TauriEvent } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from 'svelte'

  import Tab from '../components/diff/Tab.svelte'
  import { diffTabsStore, diffTabIndexStore, updateDiffTabIndexStore } from '../stores'

  async function ready() {
    await listenDragDrop()
  }
  let unlisten: UnlistenFn | undefined
  async function listenDragDrop() {
    unlisten = await getCurrent().onDragDropEvent((event: TauriEvent<DragDropEvent>) => {
      console.log(event.payload)
    })
  }

  onMount(ready)
  onDestroy(() => {
    unlisten && unlisten()
  })
</script>

<h1>Patch Hygge</h1>

<div class="wrapper">
  <div class="tabs">
    {#each $diffTabsStore as _diffTab, i}
      <h2><label><input type="radio" name="diffTabs" value={i} checked={i === $diffTabIndexStore} on:change={(event) => updateDiffTabIndexStore(Number(event.currentTarget.value))}>{(i + 1).toString()}</label></h2>
    {/each}
  </div>
  {#each $diffTabsStore as diffTab, i}
    {#if i === $diffTabIndexStore}
      <Tab oldFilepath={diffTab.oldFilepath} newFilepath={diffTab.newFilepath} />
    {/if}
  {/each}
</div>

<style>
  .wrapper {
    width: 100vw;
    height: 60vh;
  }
  .tabs {
    display: flex;
  }
</style>