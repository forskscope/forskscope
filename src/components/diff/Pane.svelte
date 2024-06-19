<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  
  const dispatch = createEventDispatcher();

  export let filepath: string
  export let diff: any // todo
  export let activeDiffBlockIndex: number | undefined
  export let innerText: string

  $: if (activeDiffBlockIndex !== undefined) {
    const el = document.querySelector(`.diff-${activeDiffBlockIndex}`)!
    el.scrollIntoView({ behavior: 'smooth' })
  }
  
  function blockClass(tag: string, index: number, activeDiffBlockIndex: number | undefined): string {
    let ret: string = 'block';
    if (['delete', 'insert', 'replace'].includes(tag)) {
      ret = `${ret} diff diff-${index.toString()} ${tag}`
      if (index === activeDiffBlockIndex) {
        ret = `${ret} active`
      }
    }
    return ret
  }

  function onInput() {
    dispatch('input')
  }
</script>

<h3>{filepath}</h3>
<div class="editor" contenteditable="true" bind:innerText={innerText} on:input={onInput}>
  {#each diff as block}
    <div class="{blockClass(block.tag, block.diff_block_index, activeDiffBlockIndex)}">
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

  .editor .block.active {
    border-top: 0.1em yellow solid;
    border-bottom: 0.1em yellow solid;
  }

  .editor .line {
    position: relative;
    width: auto;
    padding-left: 2.5em;
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
