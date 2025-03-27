<script lang="ts">
  import Header from './includes/Header.svelte'
  import Toast from '../../components/common/Toast.svelte'
  import Tabs from '../../components/tabs/Tabs.svelte'
  import { onMount } from 'svelte'
  import { _setPathSeparator } from '../../stores/file.svelte'
  import {
    activeDiffFontFamily,
    activeTheme,
    activeUiFontFamily,
    diffFontSize,
    uiFontSizeScale,
  } from '../../stores/settings/theme.svelte'

  onMount(async () => {
    await _setPathSeparator()
  })
</script>

<div
  id="app"
  class={`${$activeTheme} ${$activeUiFontFamily} ${$activeDiffFontFamily}`}
  style={`--diff-font-size: ${$diffFontSize}px; --ui-font-size: ${$uiFontSizeScale * $diffFontSize}px`}
>
  <Toast />
  <header>
    <Header />
  </header>
  <main>
    <Tabs />
  </main>
</div>
