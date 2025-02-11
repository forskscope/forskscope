<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileHandle from '../components/diff/FileHandle.svelte'
  import { filepathFromDialog } from '../components/diff/utils'

  interface LinesDiff {
    diffKind: string
    linesCount: number
    oldLines: string[]
    newLines: string[]
    replaceDiffLines: ReplaceLineDiff[]
  }

  interface ReplaceLineDiff {
    charsDiff: ReplaceCharsDiff[]
  }

  interface ReplaceCharsDiff {
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
        <button
          onclick={async () => {
            const filepath = await filepathFromDialog()
            if (filepath === null) return
            oldFilepath = filepath
            diff()
          }}
        >
          <h3>Old</h3>
          {oldFilepath}
        </button>
      </div>
      <div class="col">
        <button
          onclick={async () => {
            const filepath = await filepathFromDialog()
            if (filepath === null) return
            newFilepath = filepath
            diff()
          }}
        >
          <h3>New</h3>
          {newFilepath}
        </button>
      </div>
    </div>
    {#if !diffResult.some((x) => x.diffKind !== 'equal')}
      <!-- todo: The same files -->
      <div class="row">
        <div class="col">The files are the same</div>
        <div class="col">The files are the same</div>
      </div>
    {/if}
  {/if}
  {#each diffResult as diffBlock, i}
    <div class="row">
      <div
        class={`col old ${diffBlock.diffKind}`}
        style={`height: calc(var(--line-height) * ${diffBlock.linesCount})`}
      >
        {#if diffBlock.diffKind === 'replace'}
          <div class="replace-diff-chars">
            {#each diffBlock.replaceDiffLines as replaceDiffLine}
              <div class="diff-line">
                {#each replaceDiffLine.charsDiff as replaceDiffChars}
                  <span class={replaceDiffChars.diffKind}>{replaceDiffChars.oldStr}</span>
                {/each}
              </div>
            {/each}
          </div>
        {:else}
          {#each diffBlock.oldLines as line}
            <div class="diff-line">{line}</div>
          {/each}
        {/if}
      </div>
      <div
        class={`col new ${diffBlock.diffKind}`}
        style={`height: calc(var(--line-height) * ${diffBlock.linesCount})`}
      >
        {#if diffBlock.diffKind === 'replace'}
          <div class="replace-diff-chars">
            {#each diffBlock.replaceDiffLines as replaceDiffLine}
              <div class="diff-line">
                {#each replaceDiffLine.charsDiff as replaceDiffChars}
                  <span class={replaceDiffChars.diffKind}>{replaceDiffChars.newStr}</span>
                {/each}
              </div>
            {/each}
          </div>
        {:else}
          {#each diffBlock.newLines as line}
            <div class="diff-line">{line}</div>
          {/each}
        {/if}
      </div>
    </div>
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
    background-color: #cd9200;
  }
  .delete.new {
    background-color: #636363;
  }

  .insert.old {
    background-color: #636363;
  }
  .insert.new {
    background-color: #00cd92;
  }

  .replace.old {
    background-color: #cd0092;
  }
  .replace.new {
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
