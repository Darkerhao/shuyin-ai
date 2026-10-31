// Display only: keep original paths for persistence, identity and file operations.
export function displayPath(value: string): string {
  return value.replace(/\\\\\?\\UNC\\/gi, '\\\\').replace(/\\\\\?\\(?=[a-z]:\\)/gi, '')
}
