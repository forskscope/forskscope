<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import DragDrop from '../../../components/common/DragDrop.svelte'
  import { pushCompareSet } from '../../../stores/tabs.svelte'
  import { createCompareSetItem, type CompareSet, type CompareSetItem } from '../../../types'
  import { binaryComparisonOnly } from '../../../utils/diff.svelte'

  const filesOnDropped = async (filepaths: string[]) => {
    if (filepaths.length === 0) return

    const oldFilepath = filepaths[0]
    const oldBinaryComparisonOnly = await binaryComparisonOnly(oldFilepath)
    const oldCompareSet = {
      filepath: oldFilepath,
      binaryComparisonOnly: oldBinaryComparisonOnly,
    } as CompareSetItem

    const newCompareSet: CompareSetItem = createCompareSetItem()
    if (1 < filepaths.length) {
      const newFilepath = filepaths[1]
      const newBinaryComparisonOnly = await binaryComparisonOnly(newFilepath)
      newCompareSet.filepath = newFilepath
      newCompareSet.binaryComparisonOnly = newBinaryComparisonOnly
    }

    const compareSet = {
      old: oldCompareSet,
      new: newCompareSet,
    } as CompareSet
    pushCompareSet(compareSet)
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
