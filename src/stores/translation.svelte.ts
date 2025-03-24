
import { resolveResource } from '@tauri-apps/api/path';
import { readTextFile } from '@tauri-apps/plugin-fs';
import type { AppLanguage } from '../types';

let translation: { [key: string]: string } = $state({});

const T = (key: string): string => {
    if (key in translation) return translation[key]
    return key
}

const setTranslation = async (language: AppLanguage) => {
    const path = await resolveResource(`translations/${language}.json`)
    const updated = JSON.parse(await readTextFile(path))
    translation = updated
}

export { T, setTranslation }
