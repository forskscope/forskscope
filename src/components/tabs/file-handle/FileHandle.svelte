<script lang="ts">
  import { openFileDialog } from '../../../scripts'
  import { T } from '../../../stores/translation.svelte'
  import type { DiffFilepaths, OldOrNew } from '../../../types'

  const {
    oldFilepath,
    newFilepath,
    filepathsOnChange,
  }: {
    oldFilepath: string
    newFilepath: string
    filepathsOnChange: (diffFilepaths: DiffFilepaths) => void
  } = $props()

  let _oldFilepath: string = $state(oldFilepath)
  let _newFilepath: string = $state(newFilepath)

  const readyForDiff: boolean = $derived(0 < _oldFilepath.length && 0 < _newFilepath.length)

  const openOldFileOnClick = async (oldOrNew: OldOrNew) => {
    const filepath = await openFileDialog()
    if (!filepath) return
    if (oldOrNew === 'old') {
      _oldFilepath = filepath
    } else {
      _newFilepath = filepath
    }
  }

  const diffOnClick = () => {
    filepathsOnChange({ old: _oldFilepath, new: _newFilepath } as DiffFilepaths)
    _oldFilepath = ''
    _newFilepath = ''
  }
</script>

<div class="wrapper">
  <div class="select-file">
    <button onclick={() => openOldFileOnClick('old')}>{T('Old file')}</button>
    {#if 0 < _oldFilepath.length}
      <div>{_oldFilepath}</div>
    {:else}
      <span>({T('Not selected')})</span>
    {/if}
  </div>
  <div class="select-file">
    <button onclick={() => openOldFileOnClick('new')}>{T('New file')}</button>
    {#if 0 < _newFilepath.length}
      <div>{_newFilepath}</div>
    {:else}
      <span>({T('Not selected')})</span>
    {/if}
  </div>
  <button class="diff" onclick={diffOnClick} disabled={!readyForDiff}>{T('Compare')}</button>
</div>

<style>
  .wrapper {
    max-height: 10rem;
    padding: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .select-file {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .select-file button {
    width: 6.3rem;
  }

  .select-file div {
    font-size: 120%;
    font-weight: bold;
  }

  button.diff {
    width: 80%;
    margin-left: 10%;
  }
</style>
