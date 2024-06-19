<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { onMount } from 'svelte'

  let lineHeight = 16 // todo
  let oldContent = ''
  let newContent = ''
  let oldDiff: any[] = []
  let newDiff: any[] = []
  // let diffsDiff: any[] = []

  async function on_click() {
    let diffs: any = await invoke("diff", { oldFilepath: '', newFilepath: '' })
    oldDiff = diffs[0] as any[]
    newDiff = diffs[1] as any[]
    // diffsDiff = diffs.diff as any[]
    // console.log(diffs, oldDiff, newDiff)
  }

  onMount(async () => {
    oldContent = await invoke("file_content", { filepath: '' })
    newContent = await invoke("file_content", { filepath: '1' })
  })

  function blockClass(tag: string, index: number): string {
    const diffClass = ['delete', 'insert', 'replace'].includes(tag) ? 'diff-' + index.toString() : ''
    return diffClass
  }
  function blockBackgroundColor(tag: string): string {
    return tag === 'delete' ? 'red' : tag === 'insert' ? 'green' : tag === 'replace' ? 'purple' : 'black'
  }

  let files: any[] = []
  let dragging: boolean = false
  async function handleDrop(event: DragEvent) {
    dragging = false
    event.preventDefault()
    if (!event.dataTransfer) return
    const fileList: FileList = event.dataTransfer.files
    // console.log('Files dropped:', droppedFiles)
    const fileListArray = Array.from(fileList).slice(0, 2) // indexes 0 ~ 1 only
    files = await Promise.all(fileListArray.map(async file => ({
      name: file.name,
      path: file.webkitRelativePath || file.name,
      content: await file.text()
    })))
    console.log('Files prepared:', files)
  }
  function handleDragOver(event: DragEvent) {
    event.preventDefault()
    dragging = true
  }
  function handleDragLeave() {
    dragging = false
  }
</script>

<div class="container">
  <h1>Patch Hygge</h1>
  <a href="explorer">explorer</a>
  <button on:click={on_click}>Diff</button>

  <div class="drop-area {dragging ? 'dragging' : ''}" on:drop={handleDrop} on:dragover={handleDragOver} on:dragleave={handleDragLeave} role="region" aria-label="file dropped region" >
    Drop files here
  </div>
  <ul>
    {#each files as file}
      <li>{file.name} (Path: {file.path})</li>
    {/each}
  </ul>

  <button on:click={() => lineHeight++}>Line height</button>

  <div class="editors" style="line-height: {lineHeight}px;">
    <!-- <div class="lines-num">
      {#each Array(linesNum) as _, index}
        <div>{index + 1}</div>
      {/each}
    </div> -->
    <div class="editor" contenteditable="true">
      {#each oldDiff as block, i}
        <div class="{blockClass(block.tag, i)}" style="background-color: {blockBackgroundColor(block.tag)};">
          {#each block.lines as line}
            <div class="line">
              {#if line.length === 0}
                <br>
              {:else}
                <div>{line}</div>
              {/if}
            </div>
          {/each}
          {#each Array(block.new_lines_num) as _}
            <br>
          {/each}
        </div>
      {/each}
    </div>
    <div class="editor" contenteditable="true">
      {#each newDiff as block, i}
        <div class="{blockClass(block.tag, i)}" style="background-color: {blockBackgroundColor(block.tag)};">
          {#each block.lines as line}
            <div class="line">
              {#if line.length === 0}
                <br>
              {:else}
                <div>{line}</div>
              {/if}
            </div>
          {/each}
          {#each Array(block.new_lines_num) as _}
            <br>
          {/each}
        </div>
      {/each}
    </div>
    <!-- <div class="editor">
      <pre contenteditable="true">
        {#each diffsOld as line}
          {line}<br>
        {/each}
      </pre>
    </div>
    <div class="editor">
      <div class="overlay">
        {#each diffsDiff as diff}
          <div class="diff" style="width: 50vw; top: {diff.new_index}; height: {diff.new_len}rem;"></div>
        {/each}
      </div>
      <textarea class="new" bind:value={ newContent }></textarea>
    </div> -->
  </div>
</div>

<style>
  .editors {
    width: 100vw;
    height: 90vh;
    display: flex;
    overflow-y: auto;
  }
  .editor {
    width: 50%;
    text-align: left;
    font-family: monospace;
    /* flex-shrink: 0; */
    /* flex: 0 0 auto; */
    /* overflow-x: auto;
    white-space: nowrap; */
    counter-reset: linenumber;
  }
  .editor .line {
    position: relative;
    width: auto;
    padding-left: 2em;
    counter-increment: linenumber;
  }
  .editor .line > div {
    overflow-x: auto;
    /* white-space: nowrap; */
    white-space: pre;
  }
  .editor .line::before {
    content: counter(linenumber); /* カウンターの値を表示 */
    position: absolute;
    left: 0;
    width: 2em; /* 行番号の幅を設定 */
    text-align: right;
    margin-right: 5px; /* 行番号とテキストの間のスペース */
    color: gray;
  }

  .drop-area {
    width: 100%;
    height: 200px;
    border: 2px dashed #ccc;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: #ccc;
  }
  .drop-area.dragging {
    border-color: red;
  }
</style>
