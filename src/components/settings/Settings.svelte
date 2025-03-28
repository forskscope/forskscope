<script lang="ts">
  import {
    APP_DEFAULT_LANGUAGE,
    APP_DIFF_FONT_FAMILIES,
    APP_THEMES,
    APP_UI_FONT_FAMILIES,
  } from '../../consts'
  import {
    activeDiffFontFamily,
    activeTheme,
    activeUiFontFamily,
    diffFontSize,
    uiFontSizeScale,
  } from '../../stores/settings/theme.svelte'
  import { setTranslation, T } from '../../stores/settings/translation.svelte'
  import {
    type AppDiffFontFamily,
    type AppLanguage,
    type AppTheme,
    type AppUiFontFamily,
  } from '../../types'

  interface Selector {
    icon?: string
    title: string
    groupName: string
    items: string[]
    handler: Function
    defaultValue: string
    valueSuffix: string
  }

  let {
    closeSettings,
  }: {
    closeSettings: () => void
  } = $props()

  let language: AppLanguage = $state(APP_DEFAULT_LANGUAGE)

  const themeOnChange = (value: AppTheme) => {
    $activeTheme = value
  }

  const diffFontFamilyOnChange = (value: AppDiffFontFamily) => {
    $activeDiffFontFamily = value
  }

  const uiFontFamilyOnChange = (value: AppUiFontFamily) => {
    $activeUiFontFamily = value
  }

  const diffFontSizeOnChange = (value: number) => {
    $diffFontSize = value
  }

  const uiFontSizeScaleOnChange = (value: number) => {
    $uiFontSizeScale = value
  }

  const languageOnChange = async () => {
    await setTranslation(language)
  }

  const SELECTORS = [
    {
      icon: '🎨',
      title: 'Theme',
      groupName: 'theme',
      items: APP_THEMES,
      handler: themeOnChange,
      defaultValue: $activeTheme,
      valueSuffix: '-theme',
    } as Selector,
    {
      title: 'Diff Font',
      groupName: 'diffFontFamily',
      items: APP_DIFF_FONT_FAMILIES,
      handler: diffFontFamilyOnChange,
      defaultValue: $activeDiffFontFamily,
      valueSuffix: '-diff-font-family',
    } as Selector,
    {
      title: 'UI Font',
      groupName: 'uiFontFamily',
      items: APP_UI_FONT_FAMILIES,
      handler: uiFontFamilyOnChange,
      defaultValue: $activeUiFontFamily,
      valueSuffix: '-ui-font-family',
    } as Selector,
  ]
</script>

<!-- todo: color theme switcher -->
<div class="settings-wrapper">
  <div class="position-relative">
    <button class="close" onclick={closeSettings}>✖️</button>
    <div class="settings">
      <div class="setting">
        <h3>🌐 {T('Language')}</h3>
        <select bind:value={language} onchange={languageOnChange}>
          <option value="en">English</option>
          <option value="ja">日本語</option>
        </select>
      </div>

      {#each SELECTORS as selector}
        <div class="setting">
          <h3>{selector.icon} {T(selector.title)}</h3>
          <div>
            {#each selector.items as item}
              <label
                ><input
                  type="radio"
                  name={selector.groupName}
                  value={item}
                  onchange={(e) => {
                    selector.handler(e.currentTarget.value)
                  }}
                  checked={item === selector.defaultValue}
                />{item.replace(selector.valueSuffix, '')}</label
              >
            {/each}
          </div>
        </div>
      {/each}

      <div class="setting">
        <h3>{T('Diff Font Size')}</h3>
        <div>
          <input
            type="number"
            bind:value={$diffFontSize}
            onchange={() => {
              diffFontSizeOnChange($diffFontSize)
            }}
          />
        </div>
      </div>
      <div class="setting">
        <h3>{T('UI Font Size (Ratio to Diff)')}</h3>
        <div>
          <input
            type="number"
            step="0.05"
            min="0.2"
            max="1"
            bind:value={$uiFontSizeScale}
            onchange={() => {
              uiFontSizeScaleOnChange($uiFontSizeScale)
            }}
          />
        </div>
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
    overflow: scroll;
  }

  .settings {
    width: 100%;
    max-width: 20rem;
    height: auto;
    margin: 0.7rem auto 0;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
  }

  .close {
    position: absolute;
    right: 0.8rem;
    top: 0;
    padding: 0.3rem 0.7rem;
  }
</style>
