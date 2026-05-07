import { invoke } from "@tauri-apps/api/core";
import { safeRemoveItem } from "@/lib/browserStorage";

const LEGACY_STORAGE_KEY = "pebble-templates";

export interface EmailTemplate {
  id: string;
  name: string;
  subject: string;
  body: string;
  createdAt: number;
}

function clearLegacyTemplates() {
  safeRemoveItem(localStorage, LEGACY_STORAGE_KEY);
}

export async function listTemplates(): Promise<EmailTemplate[]> {
  const templates = await invoke<EmailTemplate[]>("list_email_templates");
  clearLegacyTemplates();
  return templates;
}

export async function saveTemplate(template: Omit<EmailTemplate, "id" | "createdAt">): Promise<EmailTemplate> {
  const saved = await invoke<EmailTemplate>("save_email_template", { template });
  clearLegacyTemplates();
  return saved;
}

export async function deleteTemplate(id: string): Promise<void> {
  await invoke<void>("delete_email_template", { id });
  clearLegacyTemplates();
}
