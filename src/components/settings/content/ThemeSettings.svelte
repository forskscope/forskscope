<script lang="ts">
  import { Palette } from 'lucide-svelte'
  import type { AppTheme, SettingsSelector, SettingsSelectorRadio } from '../../../types/settings'
  import CategorySettings from './template/CategorySettings.svelte'
  import { activeTheme } from '../../../stores/settings/theme.svelte'
  import { APP_THEMES } from '../../../consts'

  const themeOnChange = (value: AppTheme) => {
    $activeTheme = value
  }

  const selectors: SettingsSelector[] = [
    {
      type: 'radio',
      icon: Palette,
      title: 'Theme',
      groupName: 'theme',
      options: APP_THEMES,
      defaultValue: $activeTheme,
      valueSuffix: '-theme',
      onchange: themeOnChange,
    } as SettingsSelectorRadio,
  ]
</script>

<CategorySettings {selectors} />
