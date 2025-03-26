<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { openFileDialog } from '../../../scripts'
  import { T } from '../../../stores/translation.svelte'
  import type { DiffFilepaths, OldOrNew } from '../../../types'

  const DEFAULT_COMPARE_BUTTON_LABEL: string = 'Compare'
  const BINARY_MODE_COMPARE_BUTTON_LABEL: string = 'Binary Compare'

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

  let compareButtonLabel: string = $state(DEFAULT_COMPARE_BUTTON_LABEL)

  const readyToCompare: boolean = $derived(0 < _oldFilepath.length && 0 < _newFilepath.length)

  $effect(() => {
    if (_oldFilepath) {
      setCompareButtonLabel()
    }
  })
  $effect(() => {
    if (_newFilepath) {
      setCompareButtonLabel()
    }
  })

  const openOldFileOnClick = async (oldOrNew: OldOrNew) => {
    const filepath = await openFileDialog()
    if (!filepath) return
    if (oldOrNew === 'old') {
      _oldFilepath = filepath
    } else {
      _newFilepath = filepath
    }
  }

  const setCompareButtonLabel = async () => {
    if (_oldFilepath) {
      const validateOldFilepath = await invoke('validate_filepath', { filepath: _oldFilepath })
      if (!validateOldFilepath) {
        compareButtonLabel = BINARY_MODE_COMPARE_BUTTON_LABEL
        return
      }
    }

    if (_newFilepath) {
      const validateNewFilepath = await invoke('validate_filepath', { filepath: _newFilepath })
      if (!validateNewFilepath) {
        compareButtonLabel = BINARY_MODE_COMPARE_BUTTON_LABEL
        return
      }
    }

    compareButtonLabel = DEFAULT_COMPARE_BUTTON_LABEL
  }

  const compareOnClick = () => {
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
  <button class="compare" onclick={compareOnClick} disabled={!readyToCompare}
    >{T(compareButtonLabel)}</button
  >
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

  button.compare {
    width: 80%;
    margin-left: 10%;
  }
</style>
