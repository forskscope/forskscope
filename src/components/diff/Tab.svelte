<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { getCurrent, type DragDropEvent } from "@tauri-apps/api/webview"
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from 'svelte'
  import Pane from './Pane.svelte'

  export let oldFilepath
  export let newFilepath
  
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
  async function init() {
    const oldDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
    const newDiffRequest: DiffRequest = { diff_request_type: 'Content', content: '' }
    let diffs: any = await invoke("diff", { oldDiffRequest: oldDiffRequest, newDiffRequest: newDiffRequest })

    oldDiff = diffs[0] as any[]
    newDiff = diffs[1] as any[]
    blocksNum = diffs[2] as number
  }

  let unlisten: UnlistenFn | undefined
  async function listenDragDrop() {
    unlisten = await getCurrent().onDragDropEvent((event: any) => {
      console.log(event.payload)
    })
  }

  onMount(() => {
    init()
    
    listenDragDrop()
  })

  onDestroy(() => unlisten && unlisten())

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

<h2>Diff</h2>
<div class="editors" style="display: flex; flex-direction: column;">
  <div style="display: flex; position: fixed; right: 0; bottom: 0; z-index: 10000;">
    <h3>Diff blocks</h3>
    <button on:click={prevBlock} disabled={blocksNum === 0}>prev</button>
    <button on:click={nextBlock} disabled={blocksNum === 0}>next</button>
  </div>
  <div style="display: flex;">
    <Pane filepath={oldFilepath} diff={oldDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={oldInnerText} on:input={onInput} />
    <Pane filepath={newFilepath} diff={newDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={newInnerText} on:input={onInput} />
  </div>
</div>

<style>
  .editors {
    width: 100%;
    height: 90vh;
    display: flex;
    overflow-y: auto;
  }
</style>
