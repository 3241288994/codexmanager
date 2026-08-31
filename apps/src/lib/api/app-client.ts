import { invoke } from "./transport";
import type { AppSettings } from "../../types";

function normalizeSettings(value: unknown): AppSettings {
  const source = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    locale: typeof source.locale === "string" ? source.locale : "zh-CN",
    localeOptions: Array.isArray(source.localeOptions) ? source.localeOptions.map(String) : ["zh-CN", "en", "ru", "ko"],
    serviceAddr: typeof source.serviceAddr === "string" ? source.serviceAddr : "localhost:48760",
    webAuthMode: typeof source.webAuthMode === "string" ? source.webAuthMode : "none",
    webAccessPasswordConfigured: source.webAccessPasswordConfigured === true,
    theme: typeof source.theme === "string" ? source.theme : "tech",
    appearancePreset: typeof source.appearancePreset === "string" ? source.appearancePreset : "classic",
    lowTransparency: source.lowTransparency === true,
    ...source,
  } as AppSettings;
}

export const appClient = {
  async getSettings(): Promise<AppSettings> {
    return normalizeSettings(await invoke<unknown>("app_settings_get"));
  },
  async setSettings(patch: Partial<AppSettings>): Promise<AppSettings> {
    return normalizeSettings(await invoke<unknown>("app_settings_set", { patch }));
  },
};
