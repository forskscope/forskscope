<script lang="ts">
  import type { DiffFilepaths, StartupParam } from '../../types'
  import FileHandle from './file-handle/FileHandle.svelte'
  import DragDrop from '../common/DragDrop.svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'

  let {
    addDiffTab,
    filesOnDropped,
  }: {
    addDiffTab: (diffFilepaths: DiffFilepaths) => void
    filesOnDropped: (filepaths: string[]) => void
  } = $props()

  let showsFileHandle: boolean = $state(false)

  let fileHandleOldFilepath: string = $state('')
  let fileHandleNewFilepath: string = $state('')

  onMount(async () => {
    const res = (await invoke('ready').catch((error: unknown) => {
      console.error(error)
      return
    })) as StartupParam

    if (res.oldFilepath) {
      if (res.newFilepath) {
        // show startup diff tab
        addDiffTab({
          old: res.oldFilepath,
          new: res.newFilepath,
        } as DiffFilepaths)
      } else {
        // start with a file dropped
        filesOnDropped([res.oldFilepath])
      }
    }
  })

  const filepathsOnChange = (diffFilepaths: DiffFilepaths) => {
    addDiffTab(diffFilepaths)
    showsFileHandle = false
  }
</script>

<div class="drag-drop">
  <DragDrop onDrop={filesOnDropped} />
</div>

<button class="shows-file-handle" onclick={() => (showsFileHandle = !showsFileHandle)}>+</button>

<div class={`select-files ${showsFileHandle ? '' : 'd-none'}`}>
  <header>
    <button onclick={() => (showsFileHandle = !showsFileHandle)}>x</button>
  </header>

  {#key [fileHandleOldFilepath, fileHandleNewFilepath]}
    <FileHandle
      oldFilepath={fileHandleOldFilepath}
      newFilepath={fileHandleNewFilepath}
      {filepathsOnChange}
    />
  {/key}
</div>

<style>
  header {
    width: 100%;
    text-align: right;
  }

  .drag-drop {
    position: fixed;
    left: 0;
    top: 0;
    width: 100vw;
    height: 100vh;
    z-index: 0;
    pointer-events: none;
  }

  .select-files {
    position: fixed;
    left: 10vw;
    top: 10vh;
    width: 80vw;
    height: 80vh;
    padding: 0.4rem 0;
    background-color: var(--select-files-background-color);
    color: var(--select-files-text-color);
  }

  .shows-file-handle {
    padding: 0.5rem 1.5rem;
  }
</style>
