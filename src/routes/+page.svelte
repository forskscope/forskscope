<script lang="ts">
  import { getCurrent, type DragDropEvent } from "@tauri-apps/api/webview"
  import type { UnlistenFn, Event as TauriEvent } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from 'svelte'
  import DiffTab from '../components/diff/Tab.svelte'

  let unlisten: UnlistenFn | undefined
  async function listenDragDrop() {
    unlisten = await getCurrent().onDragDropEvent((event: TauriEvent<DragDropEvent>) => {
      console.log(event.payload)
    })
  }

  onMount(listenDragDrop)
  onDestroy(() => unlisten && unlisten())
</script>

<h1>Patch Hygge</h1>

<!-- todo -->
<DiffTab oldFilepath={''} newFilepath={''} />
