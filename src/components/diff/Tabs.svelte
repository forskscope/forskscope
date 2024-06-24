<script lang="ts">
  import Tab from '../../components/diff/Tab.svelte'
  import { diffTabsStore, diffTabIndexStore, updateDiffTabIndexStore, removeDiffTabsStore } from '../../stores'
</script>

<div class="tabs">
  {#each $diffTabsStore as _diffTab, i}
    <div class="tab">
      <h2>
        <input type="radio" id="diff-tab-{i + 1}" name="diff-tabs" value={i} checked={i === $diffTabIndexStore} on:change={(event) => updateDiffTabIndexStore(Number(event.currentTarget.value))}>
        <label for="diff-tab-{i + 1}">
          {(i + 1).toString()}
        </label>
      </h2>
      <button on:click={() => removeDiffTabsStore(i)}>x</button>
    </div>
  {/each}
</div>
{#each $diffTabsStore as diffTab, i}
  {#if i === $diffTabIndexStore}
    <Tab oldFilepath={diffTab.oldFilepath} newFilepath={diffTab.newFilepath} />
  {/if}
{/each}

<style>
  .tabs,
  .tabs *,
  .tab,
  .tab * {
    font-size: 0.8rem;
  }
  .tabs,
  .tab {
    display: flex;
    align-items: center;
  }
  
  .tabs {
    height: 1.8em;
  }

  .tab {
    padding: 0.4rem 0.2rem;
    background-color: grey;
  }
  .tab input {
    display: none;
  }
  .tab label {
    display: inline-block;
    min-width: 2.7rem;
  }
  .tab input:checked + label {
    color: coral;
    background-color: white;
  }
  .tab button {
    padding: 0.2rem 0.4rem;
  }
</style>