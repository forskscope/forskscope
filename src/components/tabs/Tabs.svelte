<script lang="ts">
  import {
    activateCompareSet,
    activateExplorer,
    compareSets,
    exploreIsActive,
    isActiveCompareSet,
    spliceCompareSet,
  } from '../../stores/compareSets.svelte'
  import { openMultipleFilesDialog } from '../../utils/dialog.svelte'
  import { filepathsToCompareSet } from '../../utils/diff.svelte'

  const addButtonOnClick = async () => {
    const filepaths = await openMultipleFilesDialog()
    filepathsToCompareSet(filepaths)
  }
</script>

<div class="tabs">
  <div class="tab">
    <button
      class={`explorer ${exploreIsActive() ? 'active' : ''}`}
      onclick={() => activateExplorer()}
    >
      <span>💻️</span>
    </button>
  </div>
  {#each $compareSets as compareSet, i}
    <div class="tab">
      <button
        class={`diff ${isActiveCompareSet(i) ? 'active' : ''}`}
        onclick={() => activateCompareSet(i)}
      >
        <span
          >{compareSet.new.filepath.split('/')[compareSet.new.filepath.split('/').length - 1]}</span
        >
      </button>
      <button class="close" onclick={() => spliceCompareSet(i)}>✖️</button>
    </div>
  {/each}
  <div class="tab">
    <button class="add" onclick={addButtonOnClick}>
      <span>➕️</span>
    </button>
  </div>
</div>

<style>
  .tabs {
    max-width: 100%;
    display: flex;
    overflow-x: auto;
    box-sizing: border-box;
  }
  .tab {
    border-width: 0.01rem;
    border-style: solid;
    box-sizing: border-box;
  }
  .tab.active {
    font-size: 105%;
    border-width: 0.12rem;
    border-style: solid;
  }
  .tab:hover {
    opacity: 0.87;
  }

  .tab button {
    width: 100%;
    height: 100%;
    padding: 0 0.5rem;
  }

  .tab:has(.diff) {
    padding: 0 0.5rem;
    display: flex;
  }

  .tab .diff {
    width: 7.2rem;
    white-space: pre;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .tab .close {
    width: 1.4rem;
    padding: 0;
    margin-left: 0.6rem;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    box-shadow: none;
    border: none;
    opacity: 0.87;
  }

  .tab.add {
    padding: 0.2rem;
    margin: 0;
    font-size: 0.72rem;
    border: none;
    border-radius: 0.3rem;
  }
</style>
