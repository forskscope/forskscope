export type ToastType = 'info' | 'success' | 'error'

export interface Toast {
    messages: string
    type: ToastType
    durationMilliseconds: number
}