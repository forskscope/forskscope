<script lang="ts">
  import type { DiffFilepaths } from '../../types'
  import FileHandle from './file-handle/FileHandle.svelte'
  import DragDrop from '../common/DragDrop.svelte'
  import AppTab from './AppTab.svelte'

  let diffFilepathsList: (DiffFilepaths | null)[] = $state([null])
  let activeTabIndex: number = $state(0)

  let showsFileHandle: boolean = $state(false)

  const filepathsOnChange = (diffFilepaths: DiffFilepaths) => {
    diffFilepathsList.push(diffFilepaths)
    showsFileHandle = false
  }

  const addDiffTab = (diffFilepaths: DiffFilepaths) => {
    diffFilepathsList.push(diffFilepaths)
    activeTabIndex = diffFilepathsList.length - 1
  }

  const onDrop = (filepaths: string[]) => {
    if (filepaths.length === 0) return
    const diffFilepaths =
      filepaths.length === 1
        ? ({ old: filepaths[0], new: filepaths[0] } as DiffFilepaths)
        : ({ old: filepaths[0], new: filepaths[1] } as DiffFilepaths)
    diffFilepathsList.push(diffFilepaths)
    activeTabIndex = diffFilepathsList.length - 1
  }
</script>

<button class="shows-file-handle" onclick={() => (showsFileHandle = !showsFileHandle)}>+</button>

<div class="tabs">
  <div class="headers">
    {#each diffFilepathsList as _diffFilepaths, tabIndex}
      <div class={`header ${tabIndex === activeTabIndex ? 'active' : ''}`}>
        <label><input type="radio" value={tabIndex} bind:group={activeTabIndex} />{tabIndex}</label>
        {#if 0 < tabIndex}
          <button
            onclick={() => {
              if (tabIndex === activeTabIndex) {
                activeTabIndex -= 1
              }
              diffFilepathsList.splice(tabIndex, 1)
            }}>x</button
          >
        {/if}
      </div>
    {/each}
  </div>
</div>
<div class="active-tab">
  {#each diffFilepathsList as diffFilepaths, tabIndex}
    <div class={tabIndex === activeTabIndex ? '' : 'd-none'}>
      <AppTab {diffFilepaths} diffFilepathsOnSelected={addDiffTab} />
    </div>
  {/each}
</div>

<div class="drag-drop">
  <DragDrop {onDrop} />
</div>

<div class={`select-files ${showsFileHandle ? '' : 'd-none'}`}>
  <button onclick={() => (showsFileHandle = !showsFileHandle)}>x</button>
  <FileHandle {filepathsOnChange} />
</div>

<style>
  .drag-drop {
    position: fixed;
    left: 0;
    top: 0;
    width: 100vw;
    height: 100vh;
    z-index: 0;
    pointer-events: none;
  }

  .select-files {
    position: fixed;
    left: 10vw;
    top: 10vh;
    width: 80vw;
    height: 80vh;
    padding: 0.4rem 0;
    background-color: yellow;
    color: #252525;
  }

  input[type='radio'] {
    display: none;
  }

  .headers {
    max-width: 100%;
    display: flex;
    overflow-x: auto;
  }
  .header {
    width: 5.7rem;
    display: flex;
    align-items: center;
    border: 0.01rem solid yellow;
  }
  .header.active {
    font-size: 110%;
    border-color: coral;
    border-width: 0.32rem;
  }

  .header button {
    width: 1.4rem;
    padding: 0.1rem 0.4rem;
    margin: 0 0.3rem;
    flex-grow: 0;
  }
  .header label {
    width: 100%;
    display: block;
    flex: 1;
    text-align: center;
  }
  .header label:hover {
    opacity: 0.6;
  }

  .shows-file-handle {
    padding: 0.5rem 1.5rem;
  }
</style>
