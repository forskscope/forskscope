<script lang="ts">
  import { T } from '../../../stores/settings/translation.svelte'

  const {
    showsCharsDiffs,
    charsDiffsAvailable,
    switchOldNewAvailable,
    showsCharsDiffsOnChange,
    switchOldNewOnClick,
  }: {
    showsCharsDiffs: boolean
    charsDiffsAvailable: boolean
    switchOldNewAvailable: boolean
    showsCharsDiffsOnChange: (value: boolean) => Promise<void>
    switchOldNewOnClick: () => void
  } = $props()

  let showsMenus: boolean = $state(false)
  let _showsCharsDiffs = $state(showsCharsDiffs)
</script>

<!-- todo: switch new/old -->
<label class="menus-toggle {showsMenus ? 'show' : ''}">
  <input type="checkbox" bind:checked={showsMenus} />
  <span>|||</span>
</label>
{#if showsMenus}
  <div class="menus">
    <div class="header">
      <button
        class="close"
        onclick={() => {
          showsMenus = false
        }}>✖️</button
      >
    </div>
    <div class="content">
      <label
        ><input
          type="checkbox"
          bind:checked={_showsCharsDiffs}
          onchange={() => {
            showsCharsDiffsOnChange(_showsCharsDiffs)
          }}
          disabled={!charsDiffsAvailable}
        />{T('Show chars diff')}</label
      >
      <button onclick={switchOldNewOnClick} disabled={!switchOldNewAvailable}
        >{T('Switch left/right')}</button
      >
    </div>
  </div>
{/if}

<style>
  .menus-toggle {
    padding: 0.4rem 0;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    letter-spacing: -0.2rem;
    cursor: pointer;
    transform: rotate(90deg);
    transition: transform 0.1s ease;
  }

  .menus-toggle.show {
    transform: rotate(0);
  }

  .menus {
    position: fixed;
    left: 41vw;
    top: 4.7rem;
    width: 14.4rem;
    height: 8.7rem;
    padding: 0.3rem 1.1rem;
    border-radius: 0.2rem;
    opacity: 0.93;
    z-index: 100;
  }

  .menus .header {
    height: 1.6rem;
    text-align: right;
  }

  .menus .header .close {
    font-size: 0.9rem;
    padding: 0.2rem 0.5rem;
  }

  .menus .content {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }
</style>
