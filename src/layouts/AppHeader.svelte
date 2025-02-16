<script lang="ts">
  import { APP_THEMES, type AppTheme } from '../types'

  // todo: color theme switcher
  let activeTheme: string = $state('night-theme')

  let { activeThemeOnChange }: { activeThemeOnChange: (theme: AppTheme) => void } = $props()
</script>

<h1>Patch Hygge</h1>

<!-- todo: color theme switcher -->
<div style="display: flex;">
  {#each APP_THEMES as theme}
    <label
      ><input
        type="radio"
        name="themes"
        value={theme}
        onchange={(e) => {
          activeThemeOnChange(e.currentTarget.value)
        }}
        checked={theme === activeTheme}
      />{theme}</label
    >
  {/each}
</div>
