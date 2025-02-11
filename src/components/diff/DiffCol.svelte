<script lang="ts">
  import type { LinesDiff, OldOrNew, ReplaceDetailLinesDiff, ReplaceDiffChars } from '../../types'

  const DIFF_LINE_HEIGHT: string = '1.34em'

  const {
    oldOrNew,
    linesDiffs,
  }: {
    oldOrNew: OldOrNew
    filepath: string
    linesDiffs: LinesDiff[]
    isCompletelyEqual: boolean
    filepathFromDialogOnClick: () => void
  } = $props()

  const lines = (linesDiff: LinesDiff): string[] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
  }

  const replaceDetailLines = (linesDiff: ReplaceDetailLinesDiff): ReplaceDiffChars[][] => {
    return oldOrNew === 'old' ? linesDiff.oldLines : linesDiff.newLines
  }
</script>

<div class={`diff-lines ${oldOrNew}`} style={`--line-height: ${DIFF_LINE_HEIGHT};`}>
  {#each linesDiffs as linesDiff, i}
    <div
      class={linesDiff.diffKind}
      style={`height: calc(var(--line-height) * ${linesDiff.linesCount})`}
    >
      {#if linesDiff.diffKind === 'replace'}
        <div class="replace-diff-chars">
          {#each replaceDetailLines(linesDiff.replaceDetail!) as line}
            <div class="diff-line">
              {#each line as chars}
                <span class={chars.diffKind}>{chars.chars}</span>
              {/each}
            </div>
          {/each}
        </div>
      {:else}
        {#each lines(linesDiff) as line}
          <div class="diff-line">{line}</div>
        {/each}
      {/if}
    </div>
  {/each}
</div>

<style>
  .diff-lines {
    counter-reset: line-number;
    width: fit-content;
  }

  .diff-line {
    counter-increment: line-number;
    height: var(--line-height);
  }

  .diff-line::before {
    content: counter(line-number);
    width: 3em;
    padding-right: 0.7em;
    display: inline-block;
    text-align: right;
    color: darkgray;
    z-index: 10000;
  }

  .old > .delete {
    background-color: #cd9200;
  }
  .new > .delete {
    background-color: #636363;
  }

  .old > .insert {
    background-color: #636363;
  }
  .new > .insert {
    background-color: #00cd92;
  }

  .old > .replace {
    background-color: #cd0092;
  }
  .new > .replace {
    background-color: #9200cd;
  }

  .replace-diff-chars {
    white-space: pre;
  }

  .old .replace-diff-chars .delete {
    background-color: #ffc600;
  }
  .new .replace-diff-chars .delete {
    background-color: #c6ff00;
  }
  .old .replace-diff-chars .insert {
    background-color: #00c6ff;
  }
  .new .replace-diff-chars .insert {
    background-color: #00ffc6;
  }
  .old .replace-diff-chars .replace {
    background-color: #ff00c6;
  }
  .new .replace-diff-chars .replace {
    background-color: #c600ff;
  }
</style>
