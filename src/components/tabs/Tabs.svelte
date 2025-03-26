<script lang="ts">
  import type { CompareSet } from '../../types'
  import Tab from './Tab.svelte'
  import SelectFiles from './SelectFiles.svelte'
  import { PATH_SEPARATOR } from '../../consts'

  const DEFAULT_ACTIVE_TAB_INDEX: number = 0
  const MIN_TABS_COUNT: number = 2

  interface TabControl {
    label?: string
    className?: string
    compareSet?: CompareSet
    buttonLabel?: string
    buttonOnClick?: Function
  }

  let compareSets: CompareSet[] = $state([])
  let activeTabIndex: number = $state(DEFAULT_ACTIVE_TAB_INDEX)

  let showsFileHandle: boolean = $state(false)

  const tabControls = $derived([
    { label: '💻️', className: 'explorer' } as TabControl,
    ...compareSets.map((x, i) => {
      const label =
        x!.new.filepath.split(PATH_SEPARATOR)[x!.new.filepath.split(PATH_SEPARATOR).length - 1]
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
    if (compareSets.length == MIN_TABS_COUNT) {
      activeTabIndex = DEFAULT_ACTIVE_TAB_INDEX
    }
  }

  const closeSelectFiles = () => {
    showsFileHandle = false
  }
</script>

<div class="tab-headers">
  {#each tabControls as tabControl, tabIndex}
    <label
      class={`tab-header ${tabControl.className} ${tabIndex === activeTabIndex ? 'active' : ''}`}
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

{#key showsFileHandle}
  <SelectFiles {showsFileHandle} {compareSetOnSelected} {closeSelectFiles} />
{/key}

<style>
  .tab-headers {
    height: 1.6rem;
    max-width: 100%;
    display: flex;
    overflow-x: auto;
  }
  .tab-header {
    padding: 0 0.5rem;
    display: flex;
    align-items: center;
    border-width: 0.01rem;
    border-style: solid;
  }
  .tab-header.active {
    font-size: 105%;
    border-width: 0.12rem;
    border-style: solid;
  }
  .tab-header:hover {
    opacity: 0.72;
  }

  .tab-header.diff > span {
    width: 5.7rem;
    display: inline-block;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-header span {
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .tab-header button {
    width: 1.4rem;
    padding: 0;
    margin-left: 0.6rem;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    box-shadow: none;
    border: none;
    opacity: 0.87;
  }

  .tab-header.add-diff-tab {
    border: none;
  }
  .tab-header.add-diff-tab button {
    padding: 0.2rem;
    margin: 0;
    font-size: 0.72rem;
    border-radius: 0.3rem;
  }

  .active-tab {
    height: calc(100vh - 1rem);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
</style>
