import { invoke } from "@tauri-apps/api/core"

export let PATH_SEPARATOR: string | undefined

export const _setPathSeparator = async () => {
    const res = await invoke("path_separator")
        // todo
        .catch((error: unknown) => {
            console.error(error)
            return
        })
    PATH_SEPARATOR = res as unknown as string
}
