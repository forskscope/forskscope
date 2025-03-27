<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { openFileDialog } from '../../../utils/dialog.svelte'
  import { T } from '../../../stores/settings/translation.svelte'
  import type { CompareSet, CompareSetItem, OldOrNew } from '../../../types'
  import { BINARY_MODE_COMPARE_BUTTON_LABEL, DEFAULT_COMPARE_BUTTON_LABEL } from '../../../consts'

  const {
    oldFilepath,
    newFilepath,
    compareSetOnChange,
  }: {
    oldFilepath: string
    newFilepath: string
    compareSetOnChange: (compareSet: CompareSet) => void
  } = $props()

  let _oldFilepath: string = $state(oldFilepath)
  let _oldBinaryComparisonOnly: boolean = $state(false)
  let _newFilepath: string = $state(newFilepath)
  let _newBinaryComparisonOnly: boolean = $state(false)

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
      _oldBinaryComparisonOnly = (await invoke('binary_comparison_only', {
        filepath: _oldFilepath,
      })) as boolean
      if (_oldBinaryComparisonOnly) {
        compareButtonLabel = BINARY_MODE_COMPARE_BUTTON_LABEL
        return
      }
    }

    if (_newFilepath) {
      _newBinaryComparisonOnly = (await invoke('binary_comparison_only', {
        filepath: _newFilepath,
      })) as boolean
      if (_newBinaryComparisonOnly) {
        compareButtonLabel = BINARY_MODE_COMPARE_BUTTON_LABEL
        return
      }
    }

    compareButtonLabel = DEFAULT_COMPARE_BUTTON_LABEL
  }

  const compareOnClick = () => {
    const compareSet = {
      old: {
        filepath: _oldFilepath,
        binaryComparisonOnly: _oldBinaryComparisonOnly,
      } as CompareSetItem,
      new: {
        filepath: _newFilepath,
        binaryComparisonOnly: _newBinaryComparisonOnly,
      } as CompareSetItem,
    } as CompareSet
    compareSetOnChange(compareSet)

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
