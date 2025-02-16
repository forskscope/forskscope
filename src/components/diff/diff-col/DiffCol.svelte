<script lang="ts">
  import { onMount } from 'svelte'
  import type {
    LinesDiff,
    OldOrNew,
    ReplaceDetailLinesDiff,
    ReplaceDiffChars,
  } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    oldOrNew,
    linesDiffs,
    focusedLinesDiffIndex,
  }: {
    oldOrNew: OldOrNew
    linesDiffs: LinesDiff[]
    focusedLinesDiffIndex: number | null
  } = $props()

  onMount(async () => {
    if (focusedLinesDiffIndex === null || oldOrNew !== 'old') return
    document
      .querySelector(`.${LINES_DIFF_CLASS_PREFIX}${focusedLinesDiffIndex}`)!
      .scrollIntoView({ behavior: 'smooth', inline: 'center' })
  })

  const diffLines = (linesDiff: LinesDiff, oldOrNew: OldOrNew): string[] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
  }

  const replaceDetailLines = (
    linesDiff: ReplaceDetailLinesDiff,
    oldOrNew: OldOrNew
  ): ReplaceDiffChars[][] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
  }
</script>

<div class="wrapper">
  <div class="lines-diffs" contenteditable={oldOrNew === 'new'}>
    {#each linesDiffs as linesDiff, i}
      <div
        class={`lines-diff ${linesDiff.diffKind} ${LINES_DIFF_CLASS_PREFIX}${i} ${focusedLinesDiffIndex === i ? 'focused' : ''}`}
        style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
      >
        {#if linesDiff.diffKind === 'replace'}
          <div class="replace-diff-chars">
            {#each replaceDetailLines(linesDiff.replaceDetail!, oldOrNew) as line}
              <div class="diff-line">
                {#each line as chars}
                  <span class={chars.diffKind} style={`width: ${chars.chars.length}em;`}
                    >{chars.chars}</span
                  >
                {/each}
              </div>
            {/each}
          </div>
        {:else}
          {#each diffLines(linesDiff, oldOrNew) as line}
            <div class="diff-line">{line}</div>
          {/each}
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .wrapper {
    width: 100%;
    overflow-x: auto;
    white-space: nowrap;
  }

  .lines-diffs {
    width: fit-content;
    min-width: 100%;
    min-height: 72vh;
    overflow: hidden;
    font-family: monospace;

    counter-reset: line-number;
  }

  .diff-line {
    counter-increment: line-number;
    height: var(--line-height);
  }

  .diff-line::before {
    content: counter(line-number);
    position: sticky;
    left: 0;
    top: 0;
    width: 3em;
    padding-right: 0.7em;
    display: inline-block;
    text-align: right;
    color: darkgray;
  }
</style>
