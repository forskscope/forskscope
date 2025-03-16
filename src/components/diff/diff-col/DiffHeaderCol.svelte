<script lang="ts">
  import type { OldOrNew } from '../../../types'

  const {
    oldOrNew,
    filepath,
    filepathFromDialogOnClick,
  }: {
    oldOrNew: OldOrNew
    filepath: string
    filepathFromDialogOnClick: () => void
  } = $props()

  const lastSlashIndex: number = $derived(filepath.lastIndexOf('/'))
  const parentDirsPath: string = $derived(filepath.substring(0, lastSlashIndex + 1))
  const filename: string = $derived(filepath.substring(lastSlashIndex + 1))
</script>

<button onclick={filepathFromDialogOnClick}>
  <h3>{oldOrNew.toUpperCase()}</h3>
  <div class="filepath">
    <div class="parent-dirs">{parentDirsPath}</div>
    <div class="filename">{filename}</div>
  </div>
</button>

<style>
  button {
    width: 100%;
    padding: 0.3rem 0.4rem;
    display: flex;
    align-items: center;
  }

  h3 {
    padding: 0;
    margin: 0 1rem 0 0;
    display: inline-block;
    font-size: 0.8rem;
    font-weight: normal;
  }

  .filepath {
    display: flex;
    overflow: hidden;
    align-items: center;
  }

  /* Allows shrinking */
  .parent-dirs {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Prevent from truncated */
  .filename {
    flex-shrink: 0;
    margin-left: 0.02rem;
  }
</style>
