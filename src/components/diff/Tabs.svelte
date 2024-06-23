<script lang="ts">
  import Tab from '../../components/diff/Tab.svelte'
  import { diffTabsStore, diffTabIndexStore, updateDiffTabIndexStore } from '../../stores'
</script>

<div class="tabs">
  {#each $diffTabsStore as _diffTab, i}
    <h2><label><input type="radio" name="diffTabs" value={i} checked={i === $diffTabIndexStore} on:change={(event) => updateDiffTabIndexStore(Number(event.currentTarget.value))}>{(i + 1).toString()}</label></h2>
  {/each}
</div>
{#each $diffTabsStore as diffTab, i}
  {#if i === $diffTabIndexStore}
    <Tab oldFilepath={diffTab.oldFilepath} newFilepath={diffTab.newFilepath} />
  {/if}
{/each}

<style>
  .tabs {
    display: flex;
  }
</style>