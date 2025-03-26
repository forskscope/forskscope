<script lang="ts">
  import {
    APP_DEFAULT_LANGUAGE,
    APP_DIFF_FONT_FAMILIES,
    APP_THEMES,
    APP_UI_FONT_FAMILIES,
  } from '../../consts'
  import { setTranslation, T } from '../../stores/translation.svelte'
  import {
    type AppDiffFontFamily,
    type AppLanguage,
    type AppTheme,
    type AppUiFontFamily,
  } from '../../types'

  interface Selector {
    title: string
    groupName: string
    items: string[]
    handler: Function
    defaultValue: string
    valueSuffix: string
  }

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
    close,
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
    close: () => void
  } = $props()

  let language: AppLanguage = $state(APP_DEFAULT_LANGUAGE)

  const languageOnChange = async () => {
    await setTranslation(language)
  }

  const SELECTORS = [
    {
      title: 'Theme',
      groupName: 'theme',
      items: APP_THEMES,
      handler: themeOnChange,
      defaultValue: activeTheme,
      valueSuffix: '-theme',
    } as Selector,
    {
      title: 'Diff Font',
      groupName: 'diffFontFamily',
      items: APP_DIFF_FONT_FAMILIES,
      handler: diffFontFamilyOnChange,
      defaultValue: activeDiffFontFamily,
      valueSuffix: '-diff-font-family',
    } as Selector,
    {
      title: 'UI Font',
      groupName: 'uiFontFamily',
      items: APP_UI_FONT_FAMILIES,
      handler: uiFontFamilyOnChange,
      defaultValue: activeUiFontFamily,
      valueSuffix: '-ui-font-family',
    } as Selector,
  ]
</script>

<!-- todo: color theme switcher -->
<div class="wrapper">
  <div class="position-relative">
    <div class="settings">
      <div class="setting">
        <h3>🌐 {T('Languages')}</h3>
        <select bind:value={language} onchange={languageOnChange}>
          <option value="en">English</option>
          <option value="ja">日本語</option>
        </select>
      </div>

      {#each SELECTORS as selector}
        <div class="setting">
          <h3>{T(selector.title)}</h3>
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
            bind:value={diffFontSize}
            onchange={() => {
              diffFontSizeOnChange(diffFontSize)
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
            bind:value={uiFontSizeScale}
            onchange={() => {
              uiFontSizeScaleOnChange(uiFontSizeScale)
            }}
          />
        </div>
      </div>
    </div>
    <button class="close" onclick={close}>X</button>
  </div>
</div>

<style>
  .wrapper {
    position: fixed;
    left: 0rem;
    top: 0;
    width: 100vw;
    height: 100vh;
    background-color: var(--secondary-background-color);
    color: var(--secondary-text-color);
    opacity: 0.93;
    z-index: 1000;
    overflow: scroll;
  }

  .settings {
    width: 100%;
    max-width: 20rem;
    height: auto;
    margin: 2.5rem auto 0;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
  }

  .close {
    position: absolute;
    right: 0.5rem;
    top: 0.2rem;
  }
</style>
