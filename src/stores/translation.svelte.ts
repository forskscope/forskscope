
import { resolveResource } from '@tauri-apps/api/path';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { APP_LANGUAGES, type AppLanguage } from '../types';

let translation: { [key: string]: string } = $state({});

const T = (key: string): string => {
    if (key in translation) return translation[key]
    return key
}

const setTranslation = async (language: AppLanguage) => {
    // no translation dictionary on app default language (`en` English)
    if (language === APP_LANGUAGES[0]) return

    const path = await resolveResource(`translations/${language}.json`)
    const updated = JSON.parse(await readTextFile(path))
    translation = updated
}

export { T, setTranslation }
