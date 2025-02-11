<script lang="ts">
  import type { LinesDiff, OldOrNew, ReplaceDetailLinesDiff, ReplaceDiffChars } from '../../types'

  const DIFF_LINE_HEIGHT: string = '1.34em'

  const {
    oldOrNew,
    filepath,
    isCompletelyEqual,
    filepathFromDialogOnClick,
  }: {
    oldOrNew: OldOrNew
    filepath: string
    isCompletelyEqual: boolean
    filepathFromDialogOnClick: () => void
  } = $props()
</script>

<div>
  <button onclick={filepathFromDialogOnClick}>
    <h3>{oldOrNew.toUpperCase()}</h3>
    {filepath}
  </button>
</div>

{#if isCompletelyEqual}
  <!-- todo: The same files -->
  <div>The files are the same</div>
{/if}

<style>
  h3 {
    padding: 0;
    margin: 0;
    display: inline-block;
  }
</style>
