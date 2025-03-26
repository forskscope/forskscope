<script lang="ts">
  import AppHeader from './AppHeader.svelte'
  import Toast from '../components/common/Toast.svelte'
  import Tabs from '../components/tabs/Tabs.svelte'
  import type { AppDiffFontFamily, AppTheme, AppUiFontFamily } from '../types'
  import { APP_DEFAULT_THEME } from '../consts'

  let activeTheme: AppTheme = $state(APP_DEFAULT_THEME)
  let activeDiffFontFamily: AppDiffFontFamily = $state('monospace-diff-font-family')
  let activeUiFontFamily: AppUiFontFamily = $state('sans-serif-ui-font-family')
  let diffFontSize: number = $state(16)
  let uiFontSizeScale: number = $state(1.0)

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
</script>

<div
  id="app"
  class={`${activeTheme} ${activeUiFontFamily} ${activeDiffFontFamily}`}
  style={`--diff-font-size: ${diffFontSize}px; --ui-font-size: ${uiFontSizeScale * diffFontSize}px`}
>
  <Toast />
  <header>
    <AppHeader
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
  </header>
  <main>
    <Tabs />
  </main>
</div>
