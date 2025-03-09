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
    <div
      class={`lines-diff ${LINES_DIFF_CLASS_PREFIX}${i} ${focusedLinesDiffIndex === i ? 'focused' : ''}`}
      style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
    >
      {#each linesDiff.oldLines.length < linesDiff.newLines.length ? linesDiff.newLines : linesDiff.oldLines as line, j}
        <div class="diff-line">
          {#if j === 0 && linesDiff.diffKind !== 'equal'}<button
              onclick={() => {
                replaceOnClick(i)
              }}>=></button
            >{/if}
        </div>
      {/each}
    </div>
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

  .lines-diff {
    position: relative;
  }

  .diff-line {
    counter-increment: line-number;
    height: var(--line-height);
  }

  button {
    position: absolute;
    top: 5%;
    left: 0;
    height: 90%;
    padding: 0;
    display: inline-flex;
    align-items: center;
  }
</style>
