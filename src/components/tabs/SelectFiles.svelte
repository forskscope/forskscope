<script lang="ts">
  import type { DiffFilepaths, StartupParam } from '../../types'
  import FileHandle from './file-handle/FileHandle.svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'

  let {
    addDiffTab,
    filesOnDropped,
    close,
  }: {
    addDiffTab: (diffFilepaths: DiffFilepaths) => void
    filesOnDropped: (filepaths: string[]) => void
    close: () => void
  } = $props()

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
    close()
  }
</script>

<div class="select-files">
  <header>
    <button onclick={close}>x</button>
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

  .select-files {
    position: fixed;
    left: 10vw;
    top: 10vh;
    width: 80vw;
    height: 80vh;
    padding: 0.4rem 0;
  }
</style>
