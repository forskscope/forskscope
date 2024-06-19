<script lang="ts">
  export let diff
  
  function blockClass(tag: string, index: number): string {
    const diffClasses = ['delete', 'insert', 'replace'].includes(tag) ? `diff diff-${index.toString()} ${tag}` : ''
    return `block ${diffClasses}`
  }
</script>

<div class="editor" contenteditable="true">
  {#each diff as block, i}
    <div class="{blockClass(block.tag, i)}">
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

<style>
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

  .editor .block.delete {
    background-color: red;
  }
  .editor .block.insert {
    background-color: green;
  }
  .editor .block.replace {
    background-color: purple;
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
</style>
