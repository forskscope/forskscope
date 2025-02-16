<script lang="ts">
  import AppHeader from './AppHeader.svelte'
  import Toast from '../components/common/Toast.svelte'
  import Tabs from '../components/tabs/Tabs.svelte'
  import type { AppTheme } from '../types'

  let activeTheme: AppTheme = $state('night-theme')

  const activeThemeOnChange = (theme: string) => {
    activeTheme = theme
  }
</script>

<div id="app" class={activeTheme}>
  <Toast />
  <header>
    <AppHeader {activeThemeOnChange} />
  </header>
  <main>
    <Tabs />
  </main>
</div>
