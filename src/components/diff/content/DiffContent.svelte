<script lang="ts">
  import type { OldOrNew } from '../../../types/compareSets'
  import type {
    CharsDiff,
    CharsDiffLines,
    CharsDiffResponse,
    DiffKind,
    LinesDiff,
    LinesDiffResponse,
  } from '../../../types/diff'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    oldOrNew,
    linesDiffResponse,
    charsDiffResponse,
    showsCharsDiffs,
    focusedLinesDiffIndex,
  }: {
    oldOrNew: OldOrNew
    linesDiffResponse: LinesDiffResponse | null
    charsDiffResponse: CharsDiffResponse | null
    showsCharsDiffs: boolean
    focusedLinesDiffIndex: number | null
  } = $props()

  let linesDiffs: LinesDiff[] = $derived(linesDiffResponse !== null ? linesDiffResponse.diffs : [])

  const charsDiffs: CharsDiffLines[] = $derived(
    charsDiffResponse !== null ? charsDiffResponse.diffs : []
  )

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

<div class={`diff-content ${oldOrNew}`} contenteditable={oldOrNew === 'new'}>
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

<style>
  .diff-content {
    counter-reset: line-number;
  }

  .diff-line {
    height: var(--line-height);
    white-space: nowrap;
    counter-increment: line-number;
  }

  .diff-line::before {
    content: counter(line-number);
    position: sticky;
    left: 0;
    top: 0;
    width: 3em;
    padding-right: 0.7rem;
    display: inline-block;
    text-align: right;
  }
</style>
