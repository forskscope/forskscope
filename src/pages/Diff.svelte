<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileHandle from '../components/diff/FileHandle.svelte'
  import type { LinesDiff } from '../types'
  import DiffCol from '../components/diff/DiffCol.svelte'
  import { filepathFromDialog } from '../components/diff/utils'

  let oldFilepath: string = $state('')
  let newFilepath: string = $state('')

  let showsFileHandler: boolean = $state(true)

  let linesDiffs: LinesDiff[] = $state([])

  const isCompletelyEqual = $derived(!linesDiffs.some((x) => x.diffKind !== 'equal'))

  const diff = async () => {
    invoke('diff_filepaths', { old: oldFilepath, new: newFilepath })
      .then((ret: unknown) => {
        console.log(ret) // todo
        linesDiffs = ret as LinesDiff[]
        showsFileHandler = false
      })
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  }

  const filepathsOnChange = (oldValue: string, newValue: string) => {
    oldFilepath = oldValue
    newFilepath = newValue
    diff()
  }

  const resetOnClick = () => {
    oldFilepath = ''
    newFilepath = ''
    linesDiffs = []
    showsFileHandler = true
  }
</script>

<h2>Diff</h2>

{#if showsFileHandler}
  {#key [oldFilepath, newFilepath]}
    <FileHandle {oldFilepath} {newFilepath} {filepathsOnChange} />
  {/key}
{/if}
<label
  ><input
    type="checkbox"
    bind:checked={showsFileHandler}
    disabled={linesDiffs.length === 0}
  />L</label
>
<button onclick={resetOnClick}>Reset</button>

{#if 0 < linesDiffs.length}
  <div class="row">
    <div class="col">
      <DiffCol
        oldOrNew="old"
        filepath={oldFilepath}
        {linesDiffs}
        {isCompletelyEqual}
        filepathFromDialogOnClick={async () => {
          const filepath = await filepathFromDialog()
          if (filepath === null) return
          oldFilepath = filepath
          diff()
        }}
      />
    </div>
    <div class="col">
      <DiffCol
        oldOrNew="new"
        filepath={newFilepath}
        {linesDiffs}
        {isCompletelyEqual}
        filepathFromDialogOnClick={async () => {
          const filepath = await filepathFromDialog()
          if (filepath === null) return
          newFilepath = filepath
          diff()
        }}
      />
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    gap: 0.9rem;
  }

  .row > .col {
    flex-grow: 1;
    flex-basis: 0;
  }
</style>
