import { invoke } from "@tauri-apps/api/core";
import { safeRemoveItem } from "@/lib/browserStorage";

const LEGACY_STORAGE_KEY = "pebble-signatures";

function clearLegacySignatures() {
  safeRemoveItem(localStorage, LEGACY_STORAGE_KEY);
}

export async function getSignature(accountId: string): Promise<string> {
  const signature = await invoke<string>("get_email_signature", { accountId });
  clearLegacySignatures();
  return signature;
}

export async function setSignature(accountId: string, signature: string): Promise<void> {
  await invoke<void>("set_email_signature", { accountId, signature });
  clearLegacySignatures();
}
