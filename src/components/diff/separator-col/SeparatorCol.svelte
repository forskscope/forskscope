<script lang="ts">
  import type { LinesDiff } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    linesDiffs,
    focusedLinesDiffIndex,
    replaceOnClick,
  }: {
    linesDiffs: LinesDiff[]
    focusedLinesDiffIndex: number | null
    replaceOnClick: (linesDiffIndex: number) => void
  } = $props()
</script>

<div class="lines-diffs">
  {#each linesDiffs as linesDiff, i}
    <button
      class={`lines-diff ${LINES_DIFF_CLASS_PREFIX}${i} ${focusedLinesDiffIndex === i ? 'focused' : ''}`}
      style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
      onclick={() => {
        replaceOnClick(i)
      }}
      disabled={linesDiff.diffKind === 'equal'}
    >
      {#if linesDiff.diffKind !== 'equal'}
        <span>⇨</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .lines-diffs {
    counter-reset: line-number;
    width: fit-content;
    min-width: 100%;
    height: fit-content;
    min-height: 100%;
  }

  button {
    padding: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
  }
</style>
