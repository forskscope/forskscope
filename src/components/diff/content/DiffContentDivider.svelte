<script lang="ts">
  import { ArrowRight } from 'lucide-svelte'
  import type { LinesDiff, LinesDiffResponse } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    linesDiffResponse,
    focusedLinesDiffIndex,
    mergeOnClick,
  }: {
    linesDiffResponse: LinesDiffResponse | null
    focusedLinesDiffIndex: number | null
    mergeOnClick: (index: number) => void
  } = $props()

  const linesDiffs: LinesDiff[] = $derived(
    linesDiffResponse !== null ? linesDiffResponse.diffs : []
  )
</script>

<div class="lines-diffs">
  {#each linesDiffs as linesDiff, i}
    <button
      class={`merge lines-diff ${LINES_DIFF_CLASS_PREFIX}${i} ${focusedLinesDiffIndex === i ? 'focused' : ''}`}
      style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
      onclick={() => {
        mergeOnClick(i)
      }}
      disabled={linesDiff.diffKind === 'equal'}
    >
      {#if linesDiff.diffKind !== 'equal'}
        <ArrowRight />
      {/if}
    </button>
  {/each}
</div>
