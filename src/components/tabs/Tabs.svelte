<script lang="ts">
  import type { DiffFilepaths, StartupParam } from '../../types'
  import Tab from './Tab.svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import SelectFiles from './SelectFiles.svelte'

  let diffFilepathsList: (DiffFilepaths | null)[] = $state([null])
  let activeTabIndex: number = $state(0)

  let showsFileHandle: boolean = $state(false)

  let fileHandleOldFilepath: string = $state('')
  let fileHandleNewFilepath: string = $state('')

  const addDiffTab = (diffFilepaths: DiffFilepaths) => {
    diffFilepathsList.push(diffFilepaths)
    activeTabIndex = diffFilepathsList.length - 1
  }

  const filesOnDropped = (filepaths: string[]) => {
    if (filepaths.length === 0) return

    // open file handle
    if (filepaths.length === 1) {
      if (0 < fileHandleOldFilepath.length) {
        fileHandleNewFilepath = filepaths[0]
      } else {
        fileHandleOldFilepath = filepaths[0]
        fileHandleNewFilepath = ''
      }
      showsFileHandle = true
      return
    }

    // show diff directly
    const diffFilepaths = { old: filepaths[0], new: filepaths[1] } as DiffFilepaths
    diffFilepathsList.push(diffFilepaths)
    activeTabIndex = diffFilepathsList.length - 1
  }

  const removeTab = (tabIndex: number) => {
    if (tabIndex === activeTabIndex) {
      activeTabIndex -= 1
    }
    diffFilepathsList.splice(tabIndex, 1)
  }

  onMount(async () => {
    const res = (await invoke('ready').catch((error: unknown) => {
      console.error(error)
      return
    })) as StartupParam

    if (res.oldFilepath) {
      if (res.newFilepath) {
        // show startup diff tab
        addDiffTab({
          old: res.oldFilepath,
          new: res.newFilepath,
        } as DiffFilepaths)
      } else {
        // start with a file dropped
        filesOnDropped([res.oldFilepath])
      }
    }
  })
</script>

<div class="tabs">
  <div class="headers">
    {#each diffFilepathsList as diffFilepaths, tabIndex}
      <label class={`header ${tabIndex === activeTabIndex ? 'active' : ''}`}>
        <input type="radio" value={tabIndex} bind:group={activeTabIndex} />
        <span>
          {tabIndex === 0
            ? '💻️'
            : diffFilepaths!.new.split('/')[diffFilepaths!.new.split('/').length - 1]}
        </span>
        {#if 0 < tabIndex}
          <button onclick={() => removeTab(tabIndex)}>x</button>
        {/if}
      </label>
    {/each}
  </div>
</div>
<div class="active-tab">
  {#each diffFilepathsList as diffFilepaths, tabIndex}
    <div class={tabIndex === activeTabIndex ? '' : 'd-none'}>
      <Tab
        {diffFilepaths}
        diffFilepathsOnSelected={addDiffTab}
        removeDiffTab={() => removeTab(tabIndex)}
      />
    </div>
  {/each}
</div>

<SelectFiles {addDiffTab} {filesOnDropped} />

<style>
  input[type='radio'] {
    display: none;
  }

  .headers {
    max-width: 100%;
    display: flex;
    overflow-x: auto;
  }
  .header {
    padding: 0 0.5rem;
    display: flex;
    align-items: center;
    /* todo: color vars */
    border: 0.01rem solid var(--tab-header-default-border-color);
  }
  .header.active {
    font-size: 105%;
    /* todo: color vars */
    border-color: var(--tab-header-active-border-color);
    border-width: 0.12rem;
  }
  .header:hover {
    opacity: 0.72;
  }

  .header:not(:first-of-type) > span {
    display: inline-block;
    width: 5.7rem;
    text-align: right;
  }

  .header span {
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .header button {
    width: 1.4rem;
    padding: 0.1rem 0.4rem;
    margin: 0 0 0 0.8rem;
    background: transparent;
    color: var(--tab-header-default-border-color);
    border-radius: 0.06rem;
    box-shadow: none;
    border: 0.02rem solid var(--tab-header-default-border-color);
  }
  .header button:hover {
    background-color: var(--tab-header-active-border-color);
    color: var(--tab-header-default-border-color);
    border-color: var(--tab-header-active-border-color);
  }
</style>
