"use client";

export const SUPPORTED_LOCALES = ["zh-CN", "en", "ru", "ko"] as const;

export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

export const DEFAULT_LOCALE: AppLocale = "zh-CN";

export const LOCALE_LABELS: Record<AppLocale, string> = {
  "zh-CN": "简体中文",
  en: "English",
  ru: "Русский",
  ko: "한국어",
};

export function normalizeLocale(value: unknown): AppLocale {
  const normalized = String(value || "")
    .trim()
    .toLowerCase();

  switch (normalized) {
    case "zh":
    case "zh-cn":
    case "zh_hans":
    case "zh-hans":
      return "zh-CN";
    case "en":
    case "en-us":
    case "en-gb":
      return "en";
    case "ru":
    case "ru-ru":
      return "ru";
    case "ko":
    case "ko-kr":
      return "ko";
    default:
      return DEFAULT_LOCALE;
  }
}
