<script lang="ts">
  import { filepathFromDialog } from '../../../scripts'
  import type { DiffFilepaths, OldOrNew } from '../../../types'

  const {
    filepathsOnChange,
  }: {
    filepathsOnChange: (diffFilepaths: DiffFilepaths) => void
  } = $props()

  let oldFilepath: string = $state('')
  let newFilepath: string = $state('')

  const readyForDiff: boolean = $derived(0 < oldFilepath.length && 0 < newFilepath.length)

  const openOldFileOnClick = async (oldOrNew: OldOrNew) => {
    const filepath = await filepathFromDialog()
    if (!filepath) return
    if (oldOrNew === 'old') {
      oldFilepath = filepath
    } else {
      newFilepath = filepath
    }
  }

  const diffOnClick = () => {
    filepathsOnChange({ old: oldFilepath, new: newFilepath } as DiffFilepaths)
    oldFilepath = ''
    newFilepath = ''
  }
</script>

<div class="wrapper">
  <div class="select-file">
    <button onclick={() => openOldFileOnClick('old')}>Old file</button>
    <div class={0 < oldFilepath.length ? 'selected' : ''}>{oldFilepath}</div>
  </div>
  <div class="select-file">
    <button onclick={() => openOldFileOnClick('new')}>New file</button>
    <div class={0 < newFilepath.length ? 'selected' : ''}>{newFilepath}</div>
  </div>
  <button class="diff" onclick={diffOnClick} disabled={!readyForDiff}>Diff</button>
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
  .select-file div::before {
    content: '(not selected)';
  }
  .select-file div.selected {
    font-size: 120%;
    font-weight: bold;
  }
  .select-file div.selected::before {
    content: '';
  }

  button.diff {
    width: 80%;
    margin-left: 10%;
  }
</style>
