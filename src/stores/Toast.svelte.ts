
let _toasts: ToastContent[] = $state([])

import type { ToastContent } from '../types/Toast.svelte.js'

const DEFAULT_DURATION_MILLISECONDS: number = 5000

const infoToast = (messages: string, durationMilliseconds?: number) => {
    show({
        messages,
        type: 'info',
        durationMilliseconds: durationMilliseconds ?? DEFAULT_DURATION_MILLISECONDS,
    })
}

const successToast = (messages: string, durationMilliseconds?: number) => {
    show({
        messages,
        type: 'success',
        durationMilliseconds: durationMilliseconds ?? DEFAULT_DURATION_MILLISECONDS,
    })
}

const errorToast = (messages: string, durationMilliseconds?: number) => {
    show({
        messages,
        type: 'error',
        durationMilliseconds: durationMilliseconds ?? DEFAULT_DURATION_MILLISECONDS,
    })
}

const show = (toast: ToastContent) => {
    _toasts.push(toast)
    setTimeout(hide, toast.durationMilliseconds)
}

const hide = () => {
    _toasts.shift()
}

export { infoToast, successToast, errorToast, _toasts }
