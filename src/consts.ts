import type { AppDiffFontFamily, AppLanguage, AppTheme, AppUiFontFamily } from "./types/settings"

export const APP_LANGUAGES: string[] = ['en', 'ja']
export const APP_DEFAULT_LANGUAGE: AppLanguage = APP_LANGUAGES[0]

export const APP_THEMES: string[] = ['dark-theme', 'light-theme', 'night-theme']
export const APP_DEFAULT_THEME: AppTheme = APP_THEMES[0]

export const APP_DIFF_FONT_FAMILIES: string[] = ['sans-serif-diff-font-family', 'serif-diff-font-family', 'monospace-diff-font-family']
export const APP_UI_FONT_FAMILIES: string[] = ['sans-serif-ui-font-family', 'serif-ui-font-family', 'monospace-ui-font-family']
export const APP_DEFAULT_DIFF_FONT_FAMILY: AppDiffFontFamily = APP_DIFF_FONT_FAMILIES[2]
export const APP_DEFAULT_UI_FONT_FAMILY: AppUiFontFamily = APP_UI_FONT_FAMILIES[0]

export const APP_DEFAULT_DIFF_FONT_SIZE: number = 18
export const APP_MIN_DIFF_FONT_SIZE: number = 6
export const APP_MAX_DIFF_FONT_SIZE: number = 50
export const APP_DEFAULT_UI_FONT_SCALE_SIZE: number = 0.9
export const APP_MIN_UI_FONT_SCALE_SIZE: number = 0.2
export const APP_MAX_UI_FONT_SCALE_SIZE: number = 1.5
export const APP_UI_FONT_SCALE_SIZE_STEP: number = 0.05

export const DIFF_KIND: string[] = ['equal', 'delete', 'insert', 'replace']

export const OLD_OR_NEW: string[] = ['old', 'new']

export const DEFAULT_COMPARE_BUTTON_LABEL: string = 'Compare'
export const BINARY_MODE_COMPARE_BUTTON_LABEL: string = 'Binary Compare'
