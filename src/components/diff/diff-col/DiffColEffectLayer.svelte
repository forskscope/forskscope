<script lang="ts">
  import { onMount } from 'svelte'
  import type { LinesDiff, OldOrNew } from '../../../types'
  import { LINES_DIFF_CLASS_PREFIX } from '../consts'
  import { diffLines, replaceDetailLines } from './scripts'

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
</script>

<div class={`lines-diffs ${oldOrNew}`}>
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
