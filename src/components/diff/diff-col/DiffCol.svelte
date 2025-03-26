<script lang="ts">
  import { onMount } from 'svelte'
  import type { DiffKind, LinesDiff, OldOrNew, CharsDiff, CharsDiffLines } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    oldOrNew,
    linesDiffs,
    charsDiffs,
    showsCharsDiffs,
    focusedLinesDiffIndex,
  }: {
    oldOrNew: OldOrNew
    linesDiffs: LinesDiff[]
    charsDiffs: CharsDiffLines[] | null
    showsCharsDiffs: boolean
    focusedLinesDiffIndex: number | null
  } = $props()

  onMount(async () => {
    if (focusedLinesDiffIndex === null || oldOrNew !== 'old') return
    document
      .querySelector(`.${LINES_DIFF_CLASS_PREFIX}${focusedLinesDiffIndex}`)!
      .scrollIntoView({ behavior: 'smooth' })
  })

  const diffLines = (linesDiff: LinesDiff, oldOrNew: OldOrNew): string[] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
  }

  const hasReplaceCharsDiff = (diffKind: DiffKind, diffIndex: number): boolean => {
    if (diffKind !== 'replace') return false
    return charsDiffs !== null && charsDiffs.some((x) => x.diffIndex === diffIndex)
  }

  const charsDiff = (diffIndex: number, oldOrNew: OldOrNew): CharsDiff[][] => {
    if (charsDiffs === null) return [[]]

    const charsDiff = charsDiffs.find((x) => x.diffIndex === diffIndex)!
    return oldOrNew === 'old' ? charsDiff.oldLines : charsDiff.newLines
  }
</script>

<div class="wrapper">
  <div class="lines-diffs" contenteditable={oldOrNew === 'new'}>
    {#each linesDiffs as linesDiff, i}
      <div
        class={`lines-diff ${linesDiff.diffKind} ${LINES_DIFF_CLASS_PREFIX}${i} ${focusedLinesDiffIndex === i ? 'focused' : ''}`}
        style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
      >
        {#if showsCharsDiffs && hasReplaceCharsDiff(linesDiff.diffKind, linesDiff.diffIndex)}
          <div class="chars-diff">
            {#each charsDiff(linesDiff.diffIndex, oldOrNew) as line}
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
    min-height: calc(100vh - 6.1rem);
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
    margin-right: 0.7rem;
    display: inline-block;
    text-align: center;
    color: var(--secondary-text-color);
  }
</style>
