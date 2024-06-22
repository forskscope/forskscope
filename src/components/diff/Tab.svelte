<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { onMount, onDestroy } from 'svelte'
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

  async function diff() {
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
  }

  const handleKeydown = (event: KeyboardEvent) => {
    console.log(event.key)
    switch (event.key) {
      case 'F7': prevBlock(); break;
      case 'F8': nextBlock(); break;
      default:
    }
  }

  let initialized: boolean = false
  async function ready() {
    if (!initialized) {
      await diff()
      initialized = true
    }

    window.addEventListener('keydown', handleKeydown)
  }

  onMount(ready)

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown)
  })

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

  function handleScrollToTop() {
    document.querySelector('.editors > .start')!.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }
  function handleScrollToBottom() {
    document.querySelector('.editors > .end')!.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }
</script>

<h3>Diff</h3>
<div class="editors" style="display: flex; flex-direction: column;">
  <span class='start'></span>
  <nav style="display: flex; position: fixed; right: 0; bottom: 0; z-index: 10000;">
    <div style="display: flex;">
      <h3>Diff blocks</h3>
      <button on:click={prevBlock} disabled={blocksNum === 0}>prev</button>
      <button on:click={nextBlock} disabled={blocksNum === 0}>next</button>
    </div>
    <div>
      <button on:click={handleScrollToTop}>Top</button>
      <button on:click={handleScrollToBottom}>Bottom</button>
    </div>
  </nav>
  <div class="panes">
    <div class="pane">
      <Pane filepath={oldFilepath} diff={oldDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={oldInnerText} on:input={onInput} />
    </div>
    <div class="pane">
      <Pane filepath={newFilepath} diff={newDiff} activeDiffBlockIndex={activeDiffBlockIndex} bind:innerText={newInnerText} on:input={onInput} />
    </div>
  </div>
  <span class='end'></span>
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
