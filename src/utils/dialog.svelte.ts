import { open as tauriDialogOpen, save as tauriDialogSave } from '@tauri-apps/plugin-dialog'

export const openFileDialog = async (defaultPath?: string): Promise<string | null> => {
    const filepath: string | null = await tauriDialogOpen({
        defaultPath,
        directory: false,
        multiple: false,
        filters: [
            {
                name: 'All files',
                extensions: ['*'],
            },
        ],
    })
    return filepath
}

export const openMultipleFilesDialog = async (defaultPath?: string): Promise<string[] | null> => {
    const filepaths: string[] | null = await tauriDialogOpen({
        defaultPath,
        directory: false,
        multiple: true,
        filters: [
            {
                name: 'All files',
                extensions: ['*'],
            },
        ],
    })
    return filepaths
}

export const openDirectoryDialog = async (defaultPath?: string): Promise<string | null> => {
    const dirpath: string | null = await tauriDialogOpen({
        defaultPath,
        directory: true,
        multiple: false,
    })
    return dirpath
}

export const saveFileDialog = async (defaultPath?: string): Promise<string | null> => {
    const filepath: string | null = await tauriDialogSave({ defaultPath })
    return filepath
}
