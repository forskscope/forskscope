<script lang="ts">
  import type { CompareSet } from '../../types'
  import Tab from './Tab.svelte'
  import SelectFiles from './SelectFiles.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { errorToast } from '../../stores/Toast.svelte'

  interface TabControl {
    label?: string
    className?: string
    compareSet?: CompareSet
    buttonLabel?: string
    buttonOnClick?: Function
  }

  let compareSets: CompareSet[] = $state([])
  let activeTabIndex: number = $state(0)

  let showsFileHandle: boolean = $state(false)

  const compareSetOnSelected = async (compareSet: CompareSet) => {
    compareSets.push(compareSet)
    activeTabIndex = compareSets.length
    showsFileHandle = false
  }

  const removeDiffTab = (tabIndex: number) => {
    if (tabIndex === activeTabIndex) {
      activeTabIndex -= 1
    }
    compareSets.splice(tabIndex - 1, 1)
  }

  const tabControls = $derived([
    { label: '💻️', className: 'explorer' } as TabControl,
    ...compareSets.map((x, i) => {
      const label = x!.new.filepath.split('/')[x!.new.filepath.split('/').length - 1]
      const className = 'diff'
      const compareSet = {
        old: {
          filepath: x.old.filepath,
          binaryComparisonOnly: x.old.binaryComparisonOnly,
        },
        new: {
          filepath: x.new.filepath,
          binaryComparisonOnly: x.new.binaryComparisonOnly,
        },
      } as CompareSet
      const buttonLabel = '✖️'
      const buttonOnClick = () => {
        removeDiffTab(i - 1)
      }
      return { label, className, compareSet, buttonLabel, buttonOnClick } as TabControl
    }),
    {
      className: 'add-diff-tab',
      buttonLabel: '➕️',
      buttonOnClick: () => {
        showsFileHandle = !showsFileHandle
      },
    } as TabControl,
  ])
</script>

<div class="tabs">
  <div class="headers">
    {#each tabControls as tabControl, tabIndex}
      <label
        class={`header ${tabControl.className} ${tabIndex === activeTabIndex ? 'active' : ''}`}
      >
        <input
          type="radio"
          value={tabIndex}
          bind:group={activeTabIndex}
          disabled={!tabControl.label}
        />
        <span>{tabControl.label}</span>
        {#if tabControl.buttonOnClick}
          <button
            onclick={() => {
              tabControl.buttonOnClick!()
            }}>{tabControl.buttonLabel}</button
          >
        {/if}
      </label>
    {/each}
  </div>
</div>

<div class="active-tab">
  {#each tabControls as tabControl, tabIndex}
    <div class={tabIndex === activeTabIndex ? '' : 'd-none'}>
      <Tab
        compareSet={tabControl.compareSet}
        {compareSetOnSelected}
        removeDiffTab={() => removeDiffTab(tabIndex)}
      />
    </div>
  {/each}
</div>

<SelectFiles {showsFileHandle} {compareSetOnSelected} />

<style>
  .tabs {
    height: 1.6rem;
  }

  .active-tab {
    height: calc(100vh - 1rem);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
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

  .header.diff > span {
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
  .header:not(.diff) button {
    padding: 0 0.1rem;
    margin: 0;
    border: none;
  }
  .header.explorer:hover,
  .header button:hover {
    background-color: var(--tab-header-active-border-color);
    color: var(--tab-header-default-border-color);
    border-color: var(--tab-header-active-border-color);
  }
</style>
