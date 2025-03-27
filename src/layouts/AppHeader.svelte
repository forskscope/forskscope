<script lang="ts">
  import Tooltip from '../components/common/Tooltip.svelte'
  import Settings from '../components/settings/Settings.svelte'
  import { T } from '../stores/settings/translation.svelte'

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
  <Settings {closeSettings} />
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
