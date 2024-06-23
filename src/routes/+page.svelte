<script lang="ts">
  import { getCurrent, type DragDropEvent } from "@tauri-apps/api/webview"
  import type { UnlistenFn, Event as TauriEvent } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from 'svelte'

  import Tabs from '../components/diff/Tabs.svelte'
  import { type DiffTab } from '../types'
  import { diffTabsStore, pushToDiffTabsStore } from '../stores'

  async function ready() {
    await listenDragDrop()
  }
  let unlisten: UnlistenFn | undefined
  async function listenDragDrop() {
    unlisten = await getCurrent().onDragDropEvent((event: TauriEvent<DragDropEvent>) => {
      console.log(event.payload)
      // todo
      if (event.payload.type === 'dropped' && event.payload.paths.length == 2) {
        const paths = event.payload.paths
        const oldFilepath = paths[0]
        const newFilepath = paths[1]
        const diffTab = <DiffTab>{oldFilepath: oldFilepath, newFilepath: newFilepath}
        pushToDiffTabsStore(diffTab)
      }
    })
  }

  onMount(ready)
  onDestroy(() => {
    unlisten && unlisten()
  })
</script>

<h1>Patch Hygge</h1>

<div class="wrapper">
  {#if 0 < $diffTabsStore.length }
    <Tabs />
  {:else}
    <div style="width: 100%; height: 100%; border: 2px dashed grey; display: flex; justify-content: center; align-items: center;">
      Drop two files to compare
    </div>
  {/if}
</div>

<style>
  .wrapper {
    width: 100vw;
    height: 60vh;
  }
</style>