<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let name = '';
  let dirs: string[] = [];
  let files: string[] = [];

  async function list_dir_on_click() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    const listed = await invoke("list_dir", { dirpath: '' }) as string[][]

    dirs = listed[0]
    files = listed[1]
  }
</script>

<div class="container">
  <h1>Explorer</h1>

  <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
  <button on:click={list_dir_on_click}>List dir</button>

  <ul>
    {#each dirs as dir}
      <li style="color: yellow;">{dir}</li>
    {/each}
    {#each files as file}
      <li>{file}</li>
    {/each}
  </ul>
</div>
