// common scripts

let timeout: number | undefined
export const debounce = (callback: () => void, delay: number) => {
    clearTimeout(timeout)
    timeout = setTimeout(callback, delay)
}
