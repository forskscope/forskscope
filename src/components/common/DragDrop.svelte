<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { getCurrentWebview, type DragDropEvent } from '@tauri-apps/api/webview'
  import { type UnlistenFn, type Event as TauriEvent } from '@tauri-apps/api/event'

  const { onDrop }: { onDrop: (filepaths: string[], position: { x: number; y: number }) => void } =
    $props()

  let filepath: string | undefined
  let unlistenDragDrop: UnlistenFn | undefined

  onMount(listenDragDrop)

  onDestroy(() => {
    if (unlistenDragDrop) unlistenDragDrop()
  })

  async function listenDragDrop() {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent(
      (event: TauriEvent<DragDropEvent>) => {
        if (event.payload.type === 'drop') {
          handleDrop(event.payload.paths, event.payload.position)
        }
      }
    )
  }

  function handleDrop(filepaths: string[], position: { x: number; y: number }) {
    onDrop(filepaths, position)
  }
</script>
