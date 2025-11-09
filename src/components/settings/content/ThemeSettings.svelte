<script lang="ts">
  import { Palette, Proportions, X } from 'lucide-svelte'
  import type { AppTheme, SettingsSelector, SettingsSelectorRadio } from '../../../types/settings'
  import CategorySettings from './template/CategorySettings.svelte'
  import { activeTheme, viewOrientationClass } from '../../../stores/settings/theme.svelte'
  import { APP_THEMES, APP_VIEW_ORIENTATIONS } from '../../../consts'

  const themeOnChange = (value: AppTheme) => {
    $activeTheme = value
  }

  const viewOrientationOnChange = (id: string) => {
    $viewOrientationClass = APP_VIEW_ORIENTATIONS.find((x) => x.id === id)!.viewOrientationClass
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
    {
      type: 'radio',
      icon: Proportions,
      title: 'View Orientation',
      groupName: 'view-orintation',
      options: APP_VIEW_ORIENTATIONS.map((x) => x.id),
      defaultValue: $viewOrientationClass,
      valueSuffix: '',
      onchange: viewOrientationOnChange,
    } as SettingsSelectorRadio,
  ]
</script>

<CategorySettings {selectors} />
