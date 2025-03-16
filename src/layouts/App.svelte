<script lang="ts">
  import AppHeader from './AppHeader.svelte'
  import Toast from '../components/common/Toast.svelte'
  import Tabs from '../components/tabs/Tabs.svelte'
  import type { AppDiffFontFamily, AppTheme, AppUiFontFamily } from '../types'
  import Settings from '../components/settings/Settings.svelte'

  let activeTheme: AppTheme = $state('dark-theme')
  let activeDiffFontFamily: AppDiffFontFamily = $state('monospace-diff-font-family')
  let activeUiFontFamily: AppUiFontFamily = $state('sans-serif-ui-font-family')
  let diffFontSize: number = $state(16)
  let uiFontSizeScale: number = $state(1.0)

  let showsSettings: boolean = $state(false)

  const themeOnChange = (value: AppTheme) => {
    activeTheme = value
  }

  const diffFontFamilyOnChange = (value: AppDiffFontFamily) => {
    activeDiffFontFamily = value
  }

  const uiFontFamilyOnChange = (value: AppUiFontFamily) => {
    activeUiFontFamily = value
  }

  const diffFontSizeOnChange = (value: number) => {
    diffFontSize = value
  }

  const uiFontSizeScaleOnChange = (value: number) => {
    uiFontSizeScale = value
  }

  const toggleSettings = () => {
    showsSettings = !showsSettings
  }
</script>

<div
  id="app"
  class={`${activeTheme} ${activeUiFontFamily} ${activeDiffFontFamily}`}
  style={`--diff-font-size: ${diffFontSize}px; --ui-font-size: ${uiFontSizeScale * diffFontSize}px`}
>
  <Toast />
  <header>
    <AppHeader {toggleSettings} />
  </header>
  <main>
    <Tabs />

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
      />
    </div>
  </main>
</div>
