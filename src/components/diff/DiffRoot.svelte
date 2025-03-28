<script lang="ts">
  import DiffBody from './main/DiffBody.svelte'
  import DiffFooter from './includes/DiffFooter.svelte'
  import DiffHeader from './includes/DiffHeader.svelte'
  import './style.scss'
  import { onMount } from 'svelte'
  import { invokeWithGuard } from '../../utils/backend.svelte'
  import type { BackendCommandResult, CompareSet, LinesDiffResponse } from '../../types'
  import { removeActiveCompareSet } from '../../stores/tabs.svelte'

  const { compareSet, visible }: { compareSet: CompareSet; visible: boolean } = $props()

  let linesDiffResponse: LinesDiffResponse | null = $state(null)

  // todo
  let showsCharsDiffs: boolean = $state(false)

  let focusedLinesDiffIndex: number | null = $state(null)

  const oldFilepath: string = $derived(compareSet.old.filepath)
  const newFilepath: string = $derived(compareSet.new.filepath)

  onMount(async () => {
    await diffLines()
  })

  const diffLines = async () => {
    const res: BackendCommandResult = await invokeWithGuard('diff_filepaths', {
      old: oldFilepath,
      new: newFilepath,
    })
    if (res.isError) {
      removeActiveCompareSet()
      return
    }

    linesDiffResponse = res.response as LinesDiffResponse

    focusedLinesDiffIndex = null
  }
</script>

{#if linesDiffResponse !== null}
  <div class={`diff-main ${visible ? '' : 'd-none'}`}>
    <div class="diff-header">
      <DiffHeader />
    </div>
    <div class="diff-body">
      <DiffBody {linesDiffResponse} {showsCharsDiffs} {focusedLinesDiffIndex} />
    </div>
    <div class="diff-footer">
      <DiffFooter {linesDiffResponse} />
    </div>
  </div>
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
