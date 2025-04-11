<script lang="ts">
  import { X } from 'lucide-svelte'
  import { CATEGORIES } from '../../consts'
  import { T } from '../../stores/settings/translation.svelte'
  import { type Category } from '../../types/settings'
  import ThemeSettings from './content/ThemeSettings.svelte'
  import TypographySettings from './content/TypographySettings.svelte'
  import LocaleSettings from './content/LocaleSettings.svelte'

  const {
    closeSettings,
  }: {
    closeSettings: () => void
  } = $props()

  let category: Category = $state(CATEGORIES[0])
</script>

<!-- todo: color theme switcher -->
<div class="settings-wrapper">
  <div class="position-relative">
    <button class="close" onclick={closeSettings}><X /></button>
    <div class="row">
      <div class="col categories">
        {#each CATEGORIES as x}
          <label>
            <input type="radio" bind:group={category} value={x} />
            {T(x)}
          </label>
        {/each}
      </div>
      <div class={`col settings ${category}`}>
        {#if category === 'Theme'}
          <ThemeSettings />
        {:else if category === 'Typography'}
          <TypographySettings />
        {:else if category === 'Locale'}
          <LocaleSettings />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .settings-wrapper {
    position: fixed;
    left: 0rem;
    top: 0;
    width: 100vw;
    height: 100vh;
    opacity: 0.93;
    z-index: 1000;
    overflow: auto;
  }

  .row {
    width: 100%;
    max-width: 30rem;
    height: auto;
    padding-top: 2.2rem;
    margin: 0 auto;
  }

  .categories {
    max-width: 6.3rem;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
    white-space: unset;
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 1.7rem;
  }

  .close {
    position: absolute;
    right: 1.1rem;
    top: 0.7rem;
    padding: 0.4rem 0.7rem;
  }
</style>
