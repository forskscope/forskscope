import { writable, type Writable } from "svelte/store"
import { APP_DEFAULT_DIFF_FONT_SIZE, APP_DEFAULT_DIFF_FONT_FAMILY, APP_DEFAULT_THEME, APP_DEFAULT_UI_FONT_FAMILY, APP_DEFAULT_UI_FONT_SCALE_SIZE } from "../../consts"
import type { AppDiffFontFamily, AppTheme, AppUiFontFamily } from "../../types/settings.svelte"

export let activeTheme: Writable<AppTheme> = writable(APP_DEFAULT_THEME)
export let activeDiffFontFamily: Writable<AppDiffFontFamily> = writable(APP_DEFAULT_DIFF_FONT_FAMILY)
export let activeUiFontFamily: Writable<AppUiFontFamily> = writable(APP_DEFAULT_UI_FONT_FAMILY)
export let diffFontSize: Writable<number> = writable(APP_DEFAULT_DIFF_FONT_SIZE)
export let uiFontSizeScale: Writable<number> = writable(APP_DEFAULT_UI_FONT_SCALE_SIZE)
