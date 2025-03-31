<script lang="ts">
  import { Languages } from 'lucide-svelte'
  import { APP_LANGUAGE_LABELS, APP_LANGUAGES } from '../../../consts'
  import type {
    AppLanguage,
    SettingsSelector,
    SettingsSelectorSelect,
  } from '../../../types/settings'
  import CategorySettings from './template/CategorySettings.svelte'
  import { language, setTranslation } from '../../../stores/settings/translation.svelte'

  const languageOnChange = async (value: AppLanguage) => {
    await setTranslation(value)
  }

  const selectors: SettingsSelector[] = [
    {
      type: 'select',
      icon: Languages,
      title: 'Language',
      options: APP_LANGUAGES,
      optionLabels: APP_LANGUAGE_LABELS,
      defaultValue: $language,
      onchange: languageOnChange,
    } as SettingsSelectorSelect,
  ]
</script>

<CategorySettings {selectors} />
