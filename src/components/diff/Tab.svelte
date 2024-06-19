<script lang="ts">
  import Pane from './Pane.svelte'

  export let oldDiff
  export let newDiff
  export let blocksNum: number
  export let lineHeight: number

  let activeDiffBlockIndex: number

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

<h2>Diff</h2>
<div class="editors" style="display: flex; flex-direction: column; line-height: {lineHeight}px;">
  <div style="display: flex; position: fixed; right: 0; bottom: 0; z-index: 10000;">
    <h3>Diff blocks</h3>
    <button on:click={prevBlock} disabled={blocksNum === 0}>prev</button>
    <button on:click={nextBlock} disabled={blocksNum === 0}>next</button>
  </div>
  <div style="display: flex;">
    <Pane diff={oldDiff} activeDiffBlockIndex={activeDiffBlockIndex} />
    <Pane diff={newDiff} activeDiffBlockIndex={activeDiffBlockIndex} />
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
