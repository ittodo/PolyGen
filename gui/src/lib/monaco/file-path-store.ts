// Global store for file paths per model URI
// This allows Monaco language providers to access file paths for imports resolution

const filePathMap = new Map<string, string>();

export function setFilePath(modelUri: string, filePath: string) {
  if (filePath) {
    filePathMap.set(modelUri, filePath);
  } else {
    filePathMap.delete(modelUri);
  }
}

export function getFilePath(modelUri: string): string | null {
  return filePathMap.get(modelUri) || null;
}

export function clearFilePath(modelUri: string) {
  filePathMap.delete(modelUri);
}
