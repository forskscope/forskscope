import type { APP_DIFF_FONT_FAMILIES, APP_LANGUAGES, APP_THEMES, APP_UI_FONT_FAMILIES, } from "../consts"

export type AppLanguage = typeof APP_LANGUAGES[number]

export type AppTheme = typeof APP_THEMES[number]
export type AppDiffFontFamily = typeof APP_DIFF_FONT_FAMILIES[number]
export type AppUiFontFamily = typeof APP_UI_FONT_FAMILIES[number]

