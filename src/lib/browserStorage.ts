type ReadableStorageLike = Pick<Storage, "getItem">;
type WritableStorageLike = Pick<Storage, "setItem" | "removeItem">;

function warnStorageAccess(kind: "read" | "write" | "remove", key: string, error: unknown) {
  console.warn(`[storage] Failed to ${kind} '${key}'`, error);
}

export function safeGetItem(storage: ReadableStorageLike | undefined, key: string): string | null {
  if (!storage) return null;
  try {
    return storage.getItem(key);
  } catch (error) {
    warnStorageAccess("read", key, error);
    return null;
  }
}

export function safeSetItem(storage: WritableStorageLike | undefined, key: string, value: string) {
  if (!storage) return;
  try {
    storage.setItem(key, value);
  } catch (error) {
    warnStorageAccess("write", key, error);
  }
}

export function safeRemoveItem(storage: WritableStorageLike | undefined, key: string) {
  if (!storage) return;
  try {
    storage.removeItem(key);
  } catch (error) {
    warnStorageAccess("remove", key, error);
  }
}
