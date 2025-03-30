export interface ListDirResponse {
    currentDir: string,
    dirs: string[],
    files: FileAttr[],
}

export interface FileAttr {
    name: string,
    bytesSize: string,
    humanReadableSize: string,
    lastModified: string,
    binaryComparisonOnly: boolean
}
