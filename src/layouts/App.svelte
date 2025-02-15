<script lang="ts">
  import Toast from '../components/common/Toast.svelte'
  import Diff from '../components/diff/Diff.svelte'

  // todo: color theme switcher
  let activeTheme: string = $state('night-theme')
</script>

<div id="app" class={activeTheme}>
  <Toast />
  <h1>Patch Hygge</h1>

  <!-- todo: color theme switcher -->
  <div style="display: flex;">
    {#each ['light-theme', 'dark-theme', 'night-theme', 'monokai-theme'] as theme}
      <label
        ><input
          type="radio"
          name="themes"
          value={theme}
          onchange={(e) => {
            activeTheme = e.currentTarget.value
          }}
          checked={theme === activeTheme}
        />{theme}</label
      >
    {/each}
  </div>

  <main>
    <Diff />
  </main>
</div>
