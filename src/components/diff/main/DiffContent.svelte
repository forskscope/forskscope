<script lang="ts">
  import { onMount } from 'svelte'
  import type {
    CharsDiff,
    CharsDiffLines,
    CharsDiffResponse,
    DiffKind,
    LinesDiff,
    LinesDiffResponse,
    OldOrNew,
  } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'

  const {
    oldOrNew,
    linesDiffResponse,
    charsDiffResponse,
    showsCharsDiffs,
    focusedLinesDiffIndex,
    scrollLeft,
    scrollTop,
    onScroll,
  }: {
    oldOrNew: OldOrNew
    linesDiffResponse: LinesDiffResponse
    charsDiffResponse: CharsDiffResponse | null
    showsCharsDiffs: boolean
    focusedLinesDiffIndex: number | null
    scrollLeft: number
    scrollTop: number
    onScroll: (scrollLeft: number, scrollTop: number) => void
  } = $props()

  let diffContent: HTMLDivElement | undefined

  let linesDiffs: LinesDiff[] = $derived(linesDiffResponse.diffs)

  const charsDiffs: CharsDiffLines[] = $derived(
    charsDiffResponse !== null ? charsDiffResponse.diffs : []
  )

  $effect(() => {
    if (!isNaN(scrollTop) && !isNaN(scrollLeft)) {
      if (!diffContent) return
      diffContent.scrollTo(scrollLeft, scrollTop)
    }
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

<div
  class={`diff-content ${oldOrNew}`}
  bind:this={diffContent}
  onscroll={(e) => {
    const t = e.currentTarget
    onScroll(t.scrollLeft, t.scrollTop)
  }}
  contenteditable={oldOrNew === 'new'}
>
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
