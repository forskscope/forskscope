<script lang="ts">
  import type { DiffFilepaths } from '../../types'
  import Tab from './Tab.svelte'
  import SelectFiles from './SelectFiles.svelte'

  interface TabControl {
    label?: string
    className?: string
    diffFilepaths?: DiffFilepaths
    buttonLabel?: string
    buttonOnClick?: Function
  }

  let diffFilepathsList: DiffFilepaths[] = $state([])
  let activeTabIndex: number = $state(0)

  let showsFileHandle: boolean = $state(false)

  const addDiffTab = (diffFilepaths: DiffFilepaths) => {
    diffFilepathsList.push(diffFilepaths)
    activeTabIndex = diffFilepathsList.length
    showsFileHandle = false
  }

  const removeTab = (tabIndex: number) => {
    if (tabIndex === activeTabIndex) {
      activeTabIndex -= 1
    }
    diffFilepathsList.splice(tabIndex, 1)
  }

  const tabControls = $derived([
    { label: '💻️', className: 'explorer' } as TabControl,
    ...diffFilepathsList.map((x, i) => {
      const label = x!.new.split('/')[x!.new.split('/').length - 1]
      const className = 'diff'
      const diffFilepaths = { old: x.old, new: x.new } as DiffFilepaths
      const buttonLabel = '✖️'
      const buttonOnClick = () => {
        removeTab(i - 1)
      }
      return { label, className, diffFilepaths, buttonLabel, buttonOnClick } as TabControl
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
        diffFilepaths={tabControl.diffFilepaths}
        diffFilepathsOnSelected={addDiffTab}
        removeDiffTab={() => removeTab(tabIndex)}
      />
    </div>
  {/each}
</div>

<SelectFiles {showsFileHandle} {addDiffTab} />

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
