import { invoke } from "@tauri-apps/api/core"

export const osStrFilepath = async (filename: string, dirpath: string): Promise<string> => {
    const res = await invoke('os_str_filepath', {
        filename,
        dirpath,
    })
        // todo
        .catch((error: unknown) => {
            console.error(error)
            return
        })
    return res as string
}
