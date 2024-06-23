<script lang="ts">
  import { goto } from '$app/navigation';

  import Pane from '../../components/explorer/Pane.svelte'
  import { type DiffTab } from '../../types'
  import { pushToDiffTabsStore } from '../../stores'

  const filterMinLength: number = 2
  let filterInput: string = ''
  let filter: string = ''
  $: filter = filterInput.length < filterMinLength ? '' : filterInput;
  
  function compare() {
    const diffTab = <DiffTab>{oldFilepath: `${oldDir}/${oldFilename}`, newFilepath: `${newDir}/${newFilename}`}
    pushToDiffTabsStore(diffTab)
    goto('/')
  }

  let oldDir: string = ''
  let oldFilename: string = ''
  function handleOldSelected(event: CustomEvent) {
    const {dir, filename} = event.detail as {dir: string, filename: string}
    oldDir = dir
    oldFilename = filename
  }
  
  let newDir: string = ''
  let newFilename: string = ''
  function handleNewSelected(event: CustomEvent) {
    const {dir, filename} = event.detail as {dir: string, filename: string}
    newDir = dir
    newFilename = filename
  }
</script>

<h1>Explorer</h1>

<input id="filter-input" placeholder="Filter" bind:value={filterInput} />
<button on:click={compare} disabled={!oldFilename || !newFilename}>Compare</button>

<div class="wrapper">
  <div class="explorer old">
    <Pane filter={filter} on:selectedChange={handleOldSelected} />
  </div>
  <div class="explorer new">
    <Pane filter={filter} on:selectedChange={handleNewSelected} />
  </div>
</div>

<style>
  .wrapper {
    width: 100vw;
    display: flex;
  }
  .explorer {
    width: 50%;
  }
  button:disabled {
    opacity: 0.4;
  }
</style>