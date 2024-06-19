<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { onMount } from 'svelte'
  import Pane from './Pane.svelte'

  export let oldFilepath
  export let newFilepath
  
  let oldInnerText: string
  let newInnerText: string

  let lineHeight: number = 16 // todo
  let activeDiffBlockIndex: number


  let oldDiff: any[] = []
  let newDiff: any[] = []
  let blocksNum: number = 0

  type DiffRequestType = 'Content' | 'Filepath'
  interface DiffRequest {
    diff_request_type: DiffRequestType,
    content: string,
  }

  async function diff_button_on_click() {
    const oldDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
    const newDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
    let diffs: any = await invoke("diff", { oldDiffRequest: oldDiffRequest, newDiffRequest: newDiffRequest })

    oldDiff = diffs[0] as any[]
    newDiff = diffs[1] as any[]
    blocksNum = diffs[2] as number
  }

  onMount(diff_button_on_click)

  function prevBlock() {
    if (activeDiffBlockIndex === undefined) {
      activeDiffBlockIndex = 0
      return
    }
    activeDiffBlockIndex = 0 < activeDiffBlockIndex ? activeDiffBlockIndex - 1 : 0
  }
  function nextBlock() {
    if (activeDiffBlockIndex === undefined) {
      activeDiffBlockIndex = 0
      return
    }
    const maxIndex = blocksNum - 1
    activeDiffBlockIndex = activeDiffBlockIndex < maxIndex ? activeDiffBlockIndex + 1 : maxIndex
  }
</script>

<button on:click={() => lineHeight++}>Line height</button>
<button on:click={diff_button_on_click}>Diff</button>

<h2>Diff</h2>
<div class="editors" style="display: flex; flex-direction: column; line-height: {lineHeight}px;">
  <div style="display: flex; position: fixed; right: 0; bottom: 0; z-index: 10000;">
    <h3>Diff blocks</h3>
    <button on:click={prevBlock} disabled={blocksNum === 0}>prev</button>
    <button on:click={nextBlock} disabled={blocksNum === 0}>next</button>
  </div>
  <div style="display: flex;">
    <Pane filepath={oldFilepath} diff={oldDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={oldInnerText} />
    <Pane filepath={newFilepath} diff={newDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={newInnerText} />
  </div>
</div>

{oldInnerText}

<br>

{newInnerText}

<style>
  .editors {
    width: 100%;
    height: 90vh;
    display: flex;
    overflow-y: auto;
  }
</style>
