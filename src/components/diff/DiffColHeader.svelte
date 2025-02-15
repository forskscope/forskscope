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
    <div>
      {filepath}
    </div>
  </button>
</div>

{#if isCompletelyEqual}
  <!-- todo: The same files -->
  <div>The files are the same</div>
{/if}

<style>
  button {
    width: 100%;
    display: flex;
    align-items: center;
  }

  h3 {
    padding: 0;
    margin: 0 1rem 0 0;
    display: inline-block;
  }

  button div {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: right;
  }
</style>
