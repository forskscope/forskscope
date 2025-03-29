<script lang="ts">
  import { T } from '../../../stores/settings/translation.svelte'
  import { openFileDialog } from '../../../utils/dialog.svelte'
  import Tooltip from '../../common/Tooltip.svelte'
  import type { CompareSet, OldOrNew } from '../../../types'

  const {
    oldOrNew,
    compareSet,
    filepathOnChange,
  }: {
    oldOrNew: OldOrNew
    compareSet: CompareSet
    filepathOnChange: (oldOrNew: OldOrNew, filepath: string) => void
  } = $props()

  const filepath: string = $derived(
    oldOrNew === 'old' ? compareSet.old.filepath : compareSet.new.filepath
  )

  const lastSlashIndex: number = $derived(filepath.lastIndexOf('/'))
  const parentDirsPath: string = $derived(filepath.substring(0, lastSlashIndex + 1))
  const filename: string = $derived(filepath.substring(lastSlashIndex + 1))

  const filepathOnClick = async () => {
    const filepath = await openFileDialog()
    if (filepath === null) return
    filepathOnChange(oldOrNew, filepath)
  }
</script>

<div class={`old-or-new ${oldOrNew}`}>
  <h3 class="filepath">
    <div class="parent-dirs">{parentDirsPath}</div>
    <div class="filename">{filename}</div>
  </h3>
  <Tooltip position="left" messages={T('Select file')}>
    <button onclick={filepathOnClick}>⚓️</button>
  </Tooltip>
</div>

<style>
  .old-or-new {
    display: flex;
    flex: 1;
  }

  .filepath {
    width: calc(100% - 2.4rem);
    margin-left: 0.2rem;
    margin-top: 0.2rem;
    display: inline-flex;
    overflow: hidden;
    align-items: center;
    font-size: 1rem;
    font-weight: normal;
  }

  /* allows shrinking */
  .parent-dirs {
    flex-shrink: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* prevent from truncated */
  .filename {
    flex-shrink: 0;
    margin-left: 0.02rem;
  }

  button {
    width: 2.2rem;
    padding: 0.3rem;
  }
</style>
