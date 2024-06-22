<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation';

  import { type DiffTab } from '../../types'
  import { pushToDiffTabsStore, updateDiffTabIndexStore } from '../../stores'

  // todo
  let currentDir = ''

  let filter = ''
  let dirs: string[] = []
  let files: string[] = []

  async function update(dir: string) {
    currentDir = currentDir ? `${currentDir}/${dir}` : ''

    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    const ret = await invoke("list_dir", { currentDir: currentDir }) as {current_dir: string, dirs: string[], files: string[]}

    currentDir = ret.current_dir
    dirs = ret.dirs
    files = ret.files
  }

  async function ready() {
    await update('')
  }

  onMount(ready)

  function compare() {
    pushToDiffTabsStore(<DiffTab>{oldFilepath: oldSelected, newFilepath: newSelected})
    goto('/')
  }

  let oldSelected: string | undefined
  function handleOldSelected(event: Event & { currentTarget: EventTarget & HTMLInputElement; }) {
    oldSelected = `${currentDir}/${event.currentTarget.value}`
  }
  let newSelected: string | undefined
  function handleNewSelected(event: Event & { currentTarget: EventTarget & HTMLInputElement; }) {
    newSelected = `${currentDir}/${event.currentTarget.value}`
  }
</script>

<h1>Explorer</h1>

<input id="filter-input" placeholder="Filter" bind:value={filter} />
<button on:click={compare} disabled={!oldSelected || !newSelected}>Compare</button>

<h3>Filepath: {currentDir}</h3>
<div><!-- todo -->
  old: {oldSelected},
  new: {newSelected}
</div>
<ul>
  <div class="wrapper">
    <div class="explorer old">
      <li style="color: cyan;" on:dblclick={() => update('..')}>..</li>
      {#each dirs as dir}
        {#if !filter || filter.length < 3 || dir.includes(filter)}
          <li style="color: yellow;" on:dblclick={() => update(dir)}>{dir}</li>
        {/if}
      {/each}
      {#each files as file}
        {#if !filter || filter.length < 3 || file.includes(filter)}
          <li><label><input type="radio" name="old" value={file} on:change={handleOldSelected}>{file}</label></li>
        {/if}
      {/each}
    </div>
    <div class="explorer new">
      <li style="color: cyan;" on:dblclick={() => update('..')}>..</li>
      {#each dirs as dir}
        {#if !filter || filter.length < 3 || dir.includes(filter)}
          <li style="color: yellow;" on:dblclick={() => update(dir)}>{dir}</li>
        {/if}
      {/each}
      {#each files as file}
        {#if !filter || filter.length < 3 || file.includes(filter)}
          <li><label><input type="radio" name="new" value={file} on:change={handleNewSelected}>{file}</label></li>
        {/if}
      {/each}
    </div>
  </div>
</ul>

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