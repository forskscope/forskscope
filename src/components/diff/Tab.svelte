<script lang="ts">
  import Pane from './Pane.svelte'

  export let oldDiff
  export let newDiff
  export let lineHeight
</script>

<div class="editors" style="line-height: {lineHeight}px;">
  <Pane diff={oldDiff} />
  <Pane diff={newDiff} />
</div>

<style>
  .editors {
    width: 100%;
    height: 90vh;
    display: flex;
    overflow-y: auto;
  }
</style>
