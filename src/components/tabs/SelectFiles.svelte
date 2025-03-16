<script lang="ts">
  import type { DiffFilepaths, StartupParam } from '../../types'
  import FileHandle from './file-handle/FileHandle.svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import DragDrop from '../common/DragDrop.svelte'

  let {
    showsFileHandle,
    addDiffTab,
  }: {
    showsFileHandle: boolean
    addDiffTab: (diffFilepaths: DiffFilepaths) => void
  } = $props()

  let fileHandleOldFilepath: string = $state('')
  let fileHandleNewFilepath: string = $state('')

  onMount(async () => {
    const res = (await invoke('ready').catch((error: unknown) => {
      console.error(error)
      return
    })) as StartupParam

    console.log(res)
    if (res.oldFilepath) {
      if (res.newFilepath) {
        // show startup diff tab
        addDiffTab({
          old: res.oldFilepath,
          new: res.newFilepath,
        } as DiffFilepaths)
      } else {
        // start with a file dropped
        fileHandleOldFilepath = res.oldFilepath
        showsFileHandle = true
      }
    }
  })

  const filepathsOnChange = (diffFilepaths: DiffFilepaths) => {
    addDiffTab(diffFilepaths)
    closeFileHandle()
  }

  const closeFileHandle = () => {
    showsFileHandle = false
  }

  const filesOnDropped = (filepaths: string[]) => {
    if (filepaths.length === 0) return

    // open file handle
    if (filepaths.length === 1) {
      if (0 < fileHandleOldFilepath.length) {
        fileHandleNewFilepath = filepaths[0]
      } else {
        fileHandleOldFilepath = filepaths[0]
        fileHandleNewFilepath = ''
      }
      showsFileHandle = true
      return
    }

    // show diff directly
    addDiffTab({
      old: filepaths[0],
      new: filepaths[1],
    } as DiffFilepaths)
  }
</script>

<div class={showsFileHandle ? '' : 'd-none'}>
  <div class="select-files">
    <header>
      <button onclick={closeFileHandle}>x</button>
    </header>

    {#key [fileHandleOldFilepath, fileHandleNewFilepath]}
      <FileHandle
        oldFilepath={fileHandleOldFilepath}
        newFilepath={fileHandleNewFilepath}
        {filepathsOnChange}
      />
    {/key}
  </div>
</div>

<div class="drag-drop">
  <DragDrop onDrop={filesOnDropped} />
</div>

<style>
  header {
    width: 100%;
    text-align: right;
  }

  .select-files {
    position: fixed;
    left: 10vw;
    top: 10vh;
    width: 80vw;
    height: 80vh;
    padding: 0.4rem 0;
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
</style>
