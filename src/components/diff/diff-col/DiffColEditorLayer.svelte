<script lang="ts">
  import type { LinesDiff, OldOrNew } from '../../../types'
  import { diffLines } from './scripts'

  const {
    oldOrNew,
    linesDiffs,
  }: {
    oldOrNew: OldOrNew
    linesDiffs: LinesDiff[]
  } = $props()
</script>

<div class={`lines-diffs ${oldOrNew}`} contenteditable>
  {#each linesDiffs as linesDiff}
    <div
      class={`lines-diff ${linesDiff.diffKind}`}
      style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
      contenteditable
    >
      {#each diffLines(linesDiff, oldOrNew) as line}
        <div class="diff-line" contenteditable>{line}</div>
      {/each}
    </div>
  {/each}
</div>
