<script lang="ts">
  import Tooltip from '../components/common/Tooltip.svelte'
  import Settings from '../components/settings/Settings.svelte'
  import { T } from '../stores/translation.svelte'
  import { type AppDiffFontFamily, type AppTheme, type AppUiFontFamily } from '../types'

  let {
    activeTheme,
    activeDiffFontFamily,
    activeUiFontFamily,
    diffFontSize,
    uiFontSizeScale,
    themeOnChange,
    diffFontFamilyOnChange,
    uiFontFamilyOnChange,
    diffFontSizeOnChange,
    uiFontSizeScaleOnChange,
  }: {
    activeTheme: AppTheme
    activeDiffFontFamily: AppDiffFontFamily
    activeUiFontFamily: AppUiFontFamily
    diffFontSize: number
    uiFontSizeScale: number
    themeOnChange: (value: AppTheme) => void
    diffFontFamilyOnChange: (value: AppDiffFontFamily) => void
    uiFontFamilyOnChange: (value: AppUiFontFamily) => void
    diffFontSizeOnChange: (value: number) => void
    uiFontSizeScaleOnChange: (value: number) => void
  } = $props()

  let showsSettings: boolean = $state(false)

  const toggleSettings = () => {
    showsSettings = !showsSettings
  }

  const closeSettings = () => {
    showsSettings = false
  }
</script>

<div class="headers">
  <h1>Patch Hygge</h1>

  <div class="settings">
    <Tooltip position="bottom" messages={T('Settings')}>
      <button
        onclick={() => {
          toggleSettings()
        }}
        >⚙️
      </button>
    </Tooltip>
  </div>
</div>

<div class={showsSettings ? '' : 'd-none'}>
  <Settings
    {activeTheme}
    {activeDiffFontFamily}
    {activeUiFontFamily}
    {diffFontSize}
    {uiFontSizeScale}
    {themeOnChange}
    {diffFontFamilyOnChange}
    {uiFontFamilyOnChange}
    {diffFontSizeOnChange}
    {uiFontSizeScaleOnChange}
    close={closeSettings}
  />
</div>

<style>
  .headers {
    position: fixed;
    top: 0.2rem;
    right: 0.7rem;
    display: flex;
    align-items: center;
    font-size: 1.1rem;
    gap: 0.4rem;
    z-index: 1001;
  }

  h1 {
    font-size: 0.9rem;
    opacity: 0.4;
  }

  button {
    padding: 0 0.3rem;
    opacity: 0.63;
  }
</style>
