<script lang="ts">
  import { ArrowRight } from 'lucide-svelte'
  import type { LinesDiff, LinesDiffResponse } from '../../../types'
  import { DIFF_LINE_HEIGHT, LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    linesDiffResponse,
    focusedLinesDiffIndex,
  }: {
    linesDiffResponse: LinesDiffResponse | null
    focusedLinesDiffIndex: number | null
  } = $props()

  const linesDiffs: LinesDiff[] = $derived(
    linesDiffResponse !== null ? linesDiffResponse.diffs : []
  )

  const mergeOnClick = (i: number) => {
    // todo
  }
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
