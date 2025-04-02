<script lang="ts">
  import {
    APP_DIFF_FONT_FAMILIES,
    APP_MAX_DIFF_FONT_SIZE,
    APP_MAX_UI_FONT_SIZE_SCALE,
    APP_MIN_DIFF_FONT_SIZE,
    APP_MIN_UI_FONT_SIZE_SCALE,
    APP_UI_FONT_FAMILIES,
    APP_UI_FONT_SIZE_SCALE_STEP,
  } from '../../../consts'
  import {
    activeDiffFontFamily,
    activeUiFontFamily,
    diffFontSize,
    uiFontSizeScale,
  } from '../../../stores/settings/theme.svelte'
  import type {
    AppDiffFontFamily,
    AppUiFontFamily,
    SettingsSelector,
    SettingsSelectorNumber,
    SettingsSelectorRadio,
  } from '../../../types/settings'
  import CategorySettings from './template/CategorySettings.svelte'

  const diffFontFamilyOnChange = (value: AppDiffFontFamily) => {
    $activeDiffFontFamily = value
  }

  const uiFontFamilyOnChange = (value: AppUiFontFamily) => {
    $activeUiFontFamily = value
  }

  const diffFontSizeOnChange = (value: string) => {
    $diffFontSize = Number(value)
  }

  const uiFontSizeScaleOnChange = (value: string) => {
    $uiFontSizeScale = Number(value)
  }

  const selectors: SettingsSelector[] = [
    {
      type: 'radio',
      title: 'Diff Font',
      groupName: 'diffFontFamily',
      options: APP_DIFF_FONT_FAMILIES,
      defaultValue: $activeDiffFontFamily,
      valueSuffix: '-diff-font-family',
      onchange: diffFontFamilyOnChange,
    } as SettingsSelectorRadio,
    {
      type: 'radio',
      title: 'UI Font',
      groupName: 'uiFontFamily',
      options: APP_UI_FONT_FAMILIES,
      defaultValue: $activeUiFontFamily,
      valueSuffix: '-ui-font-family',
      onchange: uiFontFamilyOnChange,
    } as SettingsSelectorRadio,
    {
      type: 'number',
      title: 'Diff Font Size',
      defaultValue: $diffFontSize,
      min: APP_MIN_DIFF_FONT_SIZE,
      max: APP_MAX_DIFF_FONT_SIZE,
      onchange: diffFontSizeOnChange,
    } as SettingsSelectorNumber,
    {
      type: 'number',
      title: 'UI Font Size (Ratio to Diff)',
      defaultValue: $uiFontSizeScale,
      min: APP_MIN_UI_FONT_SIZE_SCALE,
      max: APP_MAX_UI_FONT_SIZE_SCALE,
      step: APP_UI_FONT_SIZE_SCALE_STEP,
      onchange: uiFontSizeScaleOnChange,
    } as SettingsSelectorNumber,
  ]
</script>

<CategorySettings {selectors} />
