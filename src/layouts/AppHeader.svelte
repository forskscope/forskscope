<script lang="ts">
  import { onMount } from 'svelte'
  import Tooltip from '../components/common/Tooltip.svelte'
  import Settings from '../components/settings/Settings.svelte'
  import { setTranslation, T } from '../stores/translation.svelte.js'
  import {
    type AppDiffFontFamily,
    type AppLanguage,
    type AppTheme,
    type AppUiFontFamily,
  } from '../types'
  import { APP_DEFAULT_LANGUAGE } from '../consts'

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

  let language: AppLanguage = $state(APP_DEFAULT_LANGUAGE)

  let showsLanguages: boolean = $state(false)
  let showsSettings: boolean = $state(false)

  onMount(async () => {
    // todo : default language from settings stored
    await setTranslation(language)
  })

  const toggleLanguages = () => {
    showsLanguages = !showsLanguages
  }

  const toggleSettings = () => {
    showsSettings = !showsSettings
  }

  const languageOnChange = async () => {
    await setTranslation(language)
    showsLanguages = false
  }

  const closeSettings = () => {
    showsSettings = false
  }
</script>

<div class="headers">
  <h1>Patch Hygge</h1>

  <div class="languages">
    <Tooltip position="bottom" messages={T('Languages')}>
      <button
        onclick={() => {
          toggleLanguages()
        }}
        >🌐
      </button>
      <div class={showsLanguages ? '' : 'd-none'}>
        <select bind:value={language} onchange={languageOnChange}>
          <option value="en">English</option>
          <option value="ja">日本語</option>
        </select>
      </div>
    </Tooltip>
  </div>

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

  .languages div {
    position: absolute;
    right: 1rem;
    top: 2.2em;
  }
</style>
