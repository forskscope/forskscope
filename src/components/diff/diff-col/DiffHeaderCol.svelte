<script lang="ts">
  import { T } from '../../../stores/settings/translation.svelte'
  import { PATH_SEPARATOR } from '../../../stores/file.svelte'
  import Tooltip from '../../common/Tooltip.svelte'

  const {
    filepath,
    filepathFromDialogOnClick,
  }: {
    filepath: string
    filepathFromDialogOnClick: () => void
  } = $props()

  const lastSlashIndex: number = $derived(filepath.lastIndexOf(PATH_SEPARATOR!))
  const parentDirsPath: string = $derived(filepath.substring(0, lastSlashIndex + 1))
  const filename: string = $derived(filepath.substring(lastSlashIndex + 1))
</script>

<div class="wrapper">
  <h3 class="filepath">
    <div class="parent-dirs">{parentDirsPath}</div>
    <div class="filename">{filename}</div>
  </h3>
  <Tooltip position="left" messages={T('Select file')}>
    <button onclick={filepathFromDialogOnClick}>⚓️</button>
  </Tooltip>
</div>

<style>
  .wrapper {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 0.2rem;
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

  /* Allows shrinking */
  .parent-dirs {
    flex-shrink: 1;
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

  button {
    width: 2rem;
    padding: 0.1rem 0.4rem;
    font-size: 0.9rem;
  }
</style>
