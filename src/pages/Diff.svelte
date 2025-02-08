<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileHandle from '../components/diff/FileHandle.svelte'

  interface LinesDiff {
    diffKind: string
    linesCount: number
    oldLines: string[]
    newLines: string[]
    charsDiffIfReplace: CharsDiff[]
  }
  interface CharsDiff {
    diffKind: string
    oldStr: string
    newStr: string
  }

  const DIFF_LINE_HEIGHT: string = '1.34em'

  let oldFilepath: string = $state('')
  let newFilepath: string = $state('')

  let showsFileHandler: boolean = $state(true)

  let diffResult: LinesDiff[] = $state([])

  const diff = async () => {
    invoke('diff_filepaths', { old: oldFilepath, new: newFilepath })
      .then((ret: unknown) => {
        console.log(ret) // todo
        diffResult = ret as LinesDiff[]
        showsFileHandler = false
      })
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  }

  const filepathsOnChange = (oldValue: string, newValue: string) => {
    oldFilepath = oldValue
    newFilepath = newValue
    diff()
  }

  const resetOnClick = () => {
    oldFilepath = ''
    newFilepath = ''
    diffResult = []
    showsFileHandler = true
  }
</script>

<h2>Diff</h2>

{#if showsFileHandler}
  {#key [oldFilepath, newFilepath]}
    <FileHandle {oldFilepath} {newFilepath} {filepathsOnChange} />
  {/key}
{/if}
<label
  ><input
    type="checkbox"
    bind:checked={showsFileHandler}
    disabled={diffResult.length === 0}
  />L</label
>
<button onclick={resetOnClick}>Reset</button>

<div class="diff" style={`--line-height: ${DIFF_LINE_HEIGHT};`}>
  {#if 0 < diffResult.length}
    <div class="row">
      <div class="col">
        <h3>Old</h3>
        {oldFilepath}
      </div>
      <div class="col">
        <h3>New</h3>
        {newFilepath}
      </div>
    </div>
  {/if}
  {#each diffResult as diffBlock, i}
    <label for={`replaced-detail-${i}`}>
      <div class="row">
        <div
          class={`col old ${diffBlock.diffKind}`}
          style={`height: calc(var(--line-height) * ${diffBlock.linesCount})`}
        >
          {#each diffBlock.oldLines as line}
            <div class="diff-line">{line}</div>
          {/each}
        </div>
        <div
          class={`col new ${diffBlock.diffKind}`}
          style={`height: calc(var(--line-height) * ${diffBlock.linesCount})`}
        >
          {#each diffBlock.newLines as line}
            <div class="diff-line">{line}</div>
          {/each}
        </div>
      </div>
    </label>
    {#if diffBlock.charsDiffIfReplace}
      <label>
        <input type="checkbox" class="replaced-detail-toggle" id={`replaced-detail-${i}`} />
        <div class="replaced-detail row">
          <div class="col">
            {#each diffBlock.charsDiffIfReplace as x}
              <span class={`old ${x.diffKind}`}>{x.oldStr}</span>
            {/each}
          </div>
          <div class="col">
            {#each diffBlock.charsDiffIfReplace as x}
              <span class={`new ${x.diffKind}`}>{x.newStr}</span>
            {/each}
          </div>
        </div>
      </label>
    {/if}
  {/each}
</div>

<style>
  h3 {
    padding: 0;
    margin: 0;
    display: inline-block;
  }

  .row {
    display: flex;
    gap: 0.9rem;
  }

  .row > .col {
    flex-grow: 1;
    flex-basis: 0;
  }

  .diff {
    counter-reset: old-line-number new-line-number;
  }

  .diff-line {
    height: var(--line-height);
  }

  .old .diff-line {
    counter-increment: old-line-number;
  }

  .new .diff-line {
    counter-increment: new-line-number;
  }

  .old .diff-line::before {
    content: counter(old-line-number);
  }

  .new .diff-line::before {
    content: counter(new-line-number);
  }

  .diff-line::before {
    width: 3em;
    padding-right: 0.7em;
    display: inline-block;
    text-align: right;
    color: darkgray;
    z-index: 10000;
  }

  .delete.old {
    background-color: coral;
  }
  .delete.new {
    background-color: darkslategray;
  }

  .insert.old {
    background-color: darkslategray;
  }
  .insert.new {
    background-color: darkcyan;
  }

  .replace.old {
    background-color: purple;
  }
  .replace.new {
    background-color: darkolivegreen;
  }

  .replaced-detail {
    position: fixed;
    left: 0;
    bottom: 0;
    width: 100vw;
    height: 0;
    max-height: 7.2em;
    display: flex;
    background-color: black;
    color: white;
    opacity: 0;
  }
  .replaced-detail-toggle:checked + .replaced-detail {
    height: 70vh;
    opacity: 0.87;
    transition:
      opacity 0.4s ease,
      height 1.1s ease;
  }
  .replaced-detail .col {
    white-space: pre-line;
  }
  .replaced-detail .delete.old {
    background-color: coral;
  }
  .replaced-detail .delete.new {
    background-color: darkslategray;
  }
  .replaced-detail .insert.old {
    background-color: darkslategray;
  }
  .replaced-detail .insert.new {
    background-color: darkcyan;
  }
  .replaced-detail .replace.old {
    background-color: purple;
  }
  .replaced-detail .replace.new {
    background-color: darkolivegreen;
  }
</style>
