
import { resolveResource } from '@tauri-apps/api/path';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { type AppLanguage } from '../../types/settings';
import { APP_DEFAULT_LANGUAGE } from '../../consts';

let translation: { [key: string]: string } = $state({});

const T = (key: string): string => {
    if (key in translation) return translation[key]
    return key
}

const setTranslation = async (language: AppLanguage) => {
    // no translation dictionary on app default language
    if (language === APP_DEFAULT_LANGUAGE) {
        translation = {}
        return
    }

    // todo: translation to TOML or JSON5 manipulated in backend
    const path = await resolveResource(`translations/${language}.json`)
    const updated = JSON.parse(await readTextFile(path))
    translation = updated
}

export { T, setTranslation }
