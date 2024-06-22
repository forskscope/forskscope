<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { onMount } from 'svelte'
  import Pane from './Pane.svelte'

  export let oldFilepath: string
  export let newFilepath: string
  
  let oldInnerText: string
  let newInnerText: string

  let activeDiffBlockIndex: number

  let oldDiff: any[] = []
  let newDiff: any[] = []
  let blocksNum: number = 0

  type DiffRequestType = 'Content' | 'Filepath'
  interface DiffRequest {
    diff_request_type: DiffRequestType,
    content: string,
  }

  // todo: work w/ old|new filepath
  let initialized: boolean = false
  async function ready() {
    if (initialized) return

    let ret: {
      old_blocks: unknown[],
      new_blocks: unknown[],
      diff_blocks_num: number,
    }

    if (oldFilepath && newFilepath) {
      const params = { oldDiffRequest: <DiffRequest>{
        diff_request_type: "Filepath",
        content: oldFilepath,
      }, newDiffRequest: <DiffRequest>{
        diff_request_type: "Filepath",
        content: newFilepath,
      } }
      ret = await invoke("diff", params)
    } else {
      const oldDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
      const newDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
      const params = { oldDiffRequest: oldDiffRequest, newDiffRequest: newDiffRequest }
      ret = await invoke("diff", params)
    }

    oldDiff = ret.old_blocks
    newDiff = ret.new_blocks
    blocksNum = ret.diff_blocks_num

    initialized = true
  }

  onMount(ready)

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

  async function onInput() {
    const oldDiffRequest: DiffRequest = { diff_request_type: 'Content', content: oldInnerText }
    const newDiffRequest: DiffRequest = { diff_request_type: 'Content', content: newInnerText }
    let diffs: any = await invoke("diff", { oldDiffRequest: oldDiffRequest, newDiffRequest: newDiffRequest })

    console.log(diffs, oldDiffRequest, newInnerText)

    oldDiff = diffs[0] as any[]
    newDiff = diffs[1] as any[]
    blocksNum = diffs[2] as number
  }
</script>

<h3>Diff</h3>
<div class="editors" style="display: flex; flex-direction: column;">
  <div style="display: flex; position: fixed; right: 0; bottom: 0; z-index: 10000;">
    <h3>Diff blocks</h3>
    <button on:click={prevBlock} disabled={blocksNum === 0}>prev</button>
    <button on:click={nextBlock} disabled={blocksNum === 0}>next</button>
  </div>
  <div class="panes">
    <div class="pane">
      <Pane filepath={oldFilepath} diff={oldDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={oldInnerText} on:input={onInput} />
    </div>
    <div class="pane">
      <Pane filepath={newFilepath} diff={newDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={newInnerText} on:input={onInput} />
    </div>
  </div>
</div>

<style>
  .editors {
    width: 100%;
    height: 100%;
    display: flex;
    overflow-y: auto;
  }
  .panes {
    display: flex;
  }
  .pane {
    width: 50%;
  }
</style>
