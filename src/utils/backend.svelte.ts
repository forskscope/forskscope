import { invoke } from "@tauri-apps/api/core"
import { errorToast } from "../stores/Toast.svelte"
import type { BackendCommandResult } from "../types"

export const invokeWithGuard = async (command: string, args: Record<string, any>): Promise<BackendCommandResult> => {
    let isError = false
    const response = await invoke(command, args).catch((error: unknown) => {
        errorToast(error as string)
        isError = true
        return undefined
    })

    // todo: remove debugger
    console.log(response)

    return { response, isError } as BackendCommandResult
}
