import { type APP_DIFF_FONT_FAMILIES, type APP_LANGUAGES, type APP_THEMES, type APP_UI_FONT_FAMILIES, type CATEGORIES, } from "../consts"

export type Category = typeof CATEGORIES[number]

export type AppLanguage = typeof APP_LANGUAGES[number]

export type AppTheme = typeof APP_THEMES[number]
export type AppDiffFontFamily = typeof APP_DIFF_FONT_FAMILIES[number]
export type AppUiFontFamily = typeof APP_UI_FONT_FAMILIES[number]

export interface SettingsSelectorRadio {
    type: "radio" // type checker
    icon?: ConstructorOfATypedSvelteComponent
    title: string
    groupName: string
    options: string[]
    defaultValue: string
    valueSuffix: string // omit this when shown in view: value is defined as css class name in consts
    onchange: (value: string) => void
}

export interface SettingsSelectorSelect {
    type: "select" // type checker
    icon?: ConstructorOfATypedSvelteComponent
    title: string
    options: string[]
    optionLabels?: { [key: string]: string } // option label dicts w/ option used as key
    defaultValue: string
    onchange: (value: string) => void
}

export interface SettingsSelectorNumber {
    type: "number" // type checker
    icon?: ConstructorOfATypedSvelteComponent
    title: string
    defaultValue?: number
    min: number
    max: number
    step?: number,
    onchange: (value: string) => void
}

export type SettingsSelector = SettingsSelectorRadio | SettingsSelectorSelect | SettingsSelectorNumber
