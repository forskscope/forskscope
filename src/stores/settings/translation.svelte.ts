
import { resolveResource } from '@tauri-apps/api/path';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { type AppLanguage } from '../../types/settings';
import { APP_DEFAULT_LANGUAGE } from '../../consts';
import { writable, type Writable } from 'svelte/store';

export const language: Writable<AppLanguage> = writable(APP_DEFAULT_LANGUAGE)

let translation: { [key: string]: string } = $state({});

const T = (key: string): string => {
    if (key in translation) return translation[key]
    return key
}

const setTranslation = async (_language: AppLanguage) => {
    language.set(_language)

    // no translation dictionary on app default language
    if (_language === APP_DEFAULT_LANGUAGE) {
        translation = {}
        return
    }

    // todo: translation to TOML or JSON5 manipulated in backend
    const path = await resolveResource(`translations/${_language}.json`)
    const updated = JSON.parse(await readTextFile(path))
    translation = updated
}

export { T, setTranslation }
