<script lang="ts">
  import DragDrop from '../../../components/common/DragDrop.svelte'
  import { filepathsToListDir } from '../../../stores/explorer.svelte'
  import { invokeWithGuard } from '../../../utils/backend.svelte'
  import { filepathsToCompareSet } from '../../../utils/compareSets.svelte'

  const filesOnDropped = async (filepaths: string[]) => {
    if (filepaths.length === 0) return

    const res = await invokeWithGuard('is_file', { filepath: filepaths[0] })
    if (res.isError) return

    const isFile = res.response as boolean
    if (isFile) {
      filepathsToCompareSet(filepaths)
    } else {
      filepathsToListDir(filepaths)
    }
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
