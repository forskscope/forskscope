<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, createEventDispatcher } from 'svelte';

  export let filter: string

  const dispatch = createEventDispatcher();

  let dirs: string[] = []
  let files: string[] = []

  let dir: string = ''
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

    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    const ret = await invoke('list_dir', { currentDir: dir }) as {current_dir: string, dirs: string[], files: string[]}

    dir = ret.current_dir
    dirs = ret.dirs
    files = ret.files
  }

  async function ready() {
    await update('')
  }

  onMount(ready)
</script>

<div>
  <span style="color: grey;">{dir}</span>
  /
  <span style="color: cyan;">{filename}</span>
</div>
<ul>
  <li style="color: magenta;" on:dblclick={() => update('..')}>..</li>
  {#each dirs as dir}
    {#if !filter || dir.includes(filter)}
      <li style="color: yellow;" on:dblclick={() => update(dir)}>{dir}</li>
    {/if}
  {/each}
  {#each files as file}
    {#if !filter || file.includes(filter)}
      <li><label><input type="radio" name="old" value={file} on:change={handleSelected}>{file}</label></li>
    {/if}
  {/each}
</ul>
