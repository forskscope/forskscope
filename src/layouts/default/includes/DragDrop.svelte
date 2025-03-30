<script lang="ts">
  import DragDrop from '../../../components/common/DragDrop.svelte'
  import { filepathsToCompareSet } from '../../../utils/compareSets.svelte'

  const filesOnDropped = async (filepaths: string[]) => {
    filepathsToCompareSet(filepaths)
  }
</script>

<div class="drag-drop">
  <DragDrop onDrop={filesOnDropped} />
</div>

<style>
  .drag-drop {
    position: fixed;
    left: 0;
    top: 0;
    width: 100vw;
    height: 100vh;
    z-index: 0;
    pointer-events: none;
  }
</style>
