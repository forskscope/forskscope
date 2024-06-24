<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, createEventDispatcher } from 'svelte';

  export let paneType: 'old' | 'new'
  export let filter: string

  const dispatch = createEventDispatcher();

  let dirs: string[] = []
  let files: string[] = []

  let dir: string = ''
  let addressBarInput: string = ''
  let filename: string = ''

  let selected: string | undefined
  function handleSelected(event: Event & { currentTarget: EventTarget & HTMLInputElement; }) {
    filename = event.currentTarget.value
    dispatch('selectedChange', { dir: dir, filename: filename })
  }

  async function update(subdir: string) {
    dir = dir ? await invoke('path_join', {path1: dir, path2: subdir}) : ''
    filename = ''
    dispatch('selectedChange', { dir: dir, filename: filename })

    await list_dir(dir)
  }

  async function list_dir(target: string) {
    invoke('list_dir', { currentDir: target })
      .then((res) => {
        const ret = res as {current_dir: string, dirs: string[], files: string[]}
        dir = ret.current_dir
        addressBarInput = dir
        dirs = ret.dirs
        files = ret.files
      })
      .catch((err) => {
        // todo
        console.log(err)
        alert(`Error: ${err}`)
      })
  }

  async function ready() {
    await update('')
  }

  onMount(ready)
</script>

<div class="address-bar">
  <input placeholder="explore path" bind:value={addressBarInput}>
  <button on:click={() => list_dir(addressBarInput)}>Go to</button>
</div>
<div class="current-dir">
  <ul class="dirs">
    <li style="color: magenta;" on:dblclick={() => update('..')}>..</li>
    {#each dirs as dir}
      {#if !filter || dir.includes(filter)}
        <li style="color: yellow;" on:dblclick={() => update(dir)}>{dir}</li>
      {/if}
    {/each}
  </ul>
  <ul class="files">
    {#each files as file, i}
      {#if !filter || file.includes(filter)}
        <li>
          <input type="radio" id="{paneType}-{i + 1}" name={paneType} value={file} on:change={handleSelected}>
          <label for="{paneType}-{i + 1}">{file}</label>
        </li>
      {/if}
    {/each}
  </ul>
</div>

<style>
  .address-bar {
    width: 97%;
  }
  .address-bar input {
    width: calc(90% - 5.7rem);
  }
  .address-bar button {
    width: 5.4rem;
  }

  .current-dir ul {
    list-style: none;
  }
  .files input {
    display: none;
  }
  .files input:checked + label {
    color: cyan;
  }
</style>