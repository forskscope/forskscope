<script lang="ts">
  import { LaptopMinimal, Plus, X } from 'lucide-svelte'
  import {
    activateCompareSet,
    activateExplorer,
    compareSets,
    exploreIsActive,
    isActiveCompareSet,
    spliceCompareSet,
  } from '../../stores/compareSets.svelte'
  import { openMultipleFilesDialog } from '../../utils/dialog.svelte'
  import { filepathsToCompareSet } from '../../utils/compareSets.svelte'

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
      <span><LaptopMinimal /></span>
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
      <button class="close" onclick={() => spliceCompareSet(i)}><X /></button>
    </div>
  {/each}
  <div class="tab">
    <button class="add" onclick={addButtonOnClick}>
      <span><Plus /></span>
    </button>
  </div>
</div>

<style>
  .tabs,
  .tab {
    box-sizing: border-box;
  }

  .tabs {
    max-width: 100%;
    display: flex;
    overflow-x: auto;
  }

  .tab {
    position: relative;
    border-width: 0.01rem;
    border-style: solid;
  }
  .tab:hover {
    opacity: 0.87;
  }

  .tab .active::before {
    content: '';
    position: absolute;
    left: 0;
    bottom: 0;
    display: inline-block;
    height: 0.03rem;
    width: 100%;
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
    justify-content: flex-start;
  }
  .tab .diff span {
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
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
