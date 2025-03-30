<script lang="ts">
  import { FileDiff, Search } from 'lucide-svelte'
  import {
    newSelectedFileIndex,
    oldSelectedFileIndex,
    pushCompareSetFromSelectedFiles,
  } from '../../../stores/explorer.svelte'
  import Tooltip from '../../common/Tooltip.svelte'
  import { T } from '../../../stores/settings/translation.svelte'

  const compareIsEnabled: boolean = $derived(
    $oldSelectedFileIndex !== null && $newSelectedFileIndex !== null
  )
</script>

<div>
  <Tooltip position="top" messages={T('Select files to compare')}>
    <button onclick={pushCompareSetFromSelectedFiles} disabled={!compareIsEnabled}>
      <Search />
      <FileDiff />
    </button>
  </Tooltip>
</div>

<style>
  div {
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  button {
    width: 100%;
    padding: 0.1rem 0.2rem;
    display: flex;
    flex-direction: column;
  }
</style>
