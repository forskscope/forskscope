<script lang="ts">
  import type { CompareSet, CompareSetItem } from '../../types'
  import FileHandle from './file-handle/FileHandle.svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import DragDrop from '../common/DragDrop.svelte'

  let {
    showsFileHandle,
    compareSetOnSelected,
  }: {
    showsFileHandle: boolean
    compareSetOnSelected: (compareSet: CompareSet) => void
  } = $props()

  let fileHandleOldFilepath: string = $state('')
  let fileHandleNewFilepath: string = $state('')

  onMount(async () => {
    const res = await invoke('ready').catch((error: unknown) => {
      console.error(error)
      return
    })

    console.log(res)

    const compareSet = res as CompareSet
    if (0 < compareSet.old.filepath.length) {
      if (0 < compareSet.new.filepath.length) {
        // show startup diff tab
        compareSetOnSelected(compareSet)
      } else {
        // start with a file dropped
        fileHandleOldFilepath = compareSet.old.filepath
        showsFileHandle = true
      }
    }
  })

  const compareSetOnChange = (compareSet: CompareSet) => {
    compareSetOnSelected(compareSet)
    closeFileHandle()
  }

  const closeFileHandle = () => {
    showsFileHandle = false
  }

  const filesOnDropped = async (filepaths: string[]) => {
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
    const oldFilepath = filepaths[0]
    const oldBinaryComparisonOnly = await invoke('binary_comparison_only', {
      filepath: oldFilepath,
    }).catch((error: unknown) => {
      console.error(error)
      return
    })
    const newFilepath = filepaths[1]
    const newBinaryComparisonOnly = await invoke('binary_comparison_only', {
      filepath: newFilepath,
    }).catch((error: unknown) => {
      console.error(error)
      return
    })

    compareSetOnSelected({
      old: {
        filepath: oldFilepath,
        binaryComparisonOnly: oldBinaryComparisonOnly,
      } as CompareSetItem,
      new: {
        filepath: newFilepath,
        binaryComparisonOnly: newBinaryComparisonOnly,
      } as CompareSetItem,
    } as CompareSet)
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
        {compareSetOnChange}
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
