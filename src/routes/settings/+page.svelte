<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrent, type DragDropEvent } from "@tauri-apps/api/webview"
  
  let x: number
  let y: number
  let paths: string[] = []
  onMount(async () => {
    await getCurrent().onDragDropEvent((event: any) => {
      console.log(event.payload)

      switch (event.payload.type) {
        case 'dragged': console.log('File dragged'); break
        case 'dropped': {
          console.log('File dropped')
          x = event.payload.position.x
          y = event.payload.position.y
          paths = event.payload.paths as string[]
          break
        }
        case 'cancelled': console.log('File drop cancelled'); break
        default: console.log('Unexpected event about file dragged or dropped')
      }
    })
  })
</script>

<h1>Settings</h1>
{#if x && y}
  <div>Dropped: x = {x ?? ''}, y = {y ?? ''}</div>
{/if}
<ul>
  {#each paths as path}
    <li>{path}
  {/each}
</ul>

todo: font size, diff algorithm, ...