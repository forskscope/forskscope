<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import Tab from '../components/diff/Tab.svelte'

  let lineHeight = 16 // todo
  let oldDiff: any[] = []
  let newDiff: any[] = []
  let blocksNum: number = 0

  async function diff_button_on_click() {
    let diffs: any = await invoke("diff", { oldFilepath: '', newFilepath: '' })
    oldDiff = diffs[0] as any[]
    newDiff = diffs[1] as any[]
    blocksNum = diffs[2] as number
  }
</script>

<div class="container">
  <h1>Patch Hygge</h1>
  
  <button on:click={() => lineHeight++}>Line height</button>
  <button on:click={diff_button_on_click}>Diff</button>

  <Tab oldDiff={oldDiff} newDiff={newDiff} blocksNum={blocksNum} lineHeight={lineHeight} />
</div>

