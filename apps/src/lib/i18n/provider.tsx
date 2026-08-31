"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { appClient } from "@/lib/api/app-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useAppStore } from "@/lib/store/useAppStore";
import { AppLocale, DEFAULT_LOCALE, LOCALE_LABELS, normalizeLocale, SUPPORTED_LOCALES } from "./config";
import { translate } from "./messages";

type TranslationValues = Record<string, string | number>;

type I18nContextValue = {
  locale: AppLocale;
  localeOptions: AppLocale[];
  isSwitchingLocale: boolean;
  setLocale: (locale: AppLocale) => Promise<void>;
  t: (message: string, values?: TranslationValues) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const storedLocale = useAppStore((state) => state.appSettings.locale);
  const storedLocaleOptions = useAppStore((state) => state.appSettings.localeOptions);
  const setAppSettings = useAppStore((state) => state.setAppSettings);
  const [isSwitchingLocale, setIsSwitchingLocale] = useState(false);
  const locale = normalizeLocale(storedLocale);
  const localeOptions = useMemo(
    () => (storedLocaleOptions?.length ? storedLocaleOptions : SUPPORTED_LOCALES).map(normalizeLocale),
    [storedLocaleOptions],
  );

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    document.documentElement.lang = locale;
  }, [locale]);

  const t = useMemo(
    () => (message: string, values?: TranslationValues) => translate(locale, message, values),
    [locale],
  );

  const setLocale = useCallback(async (nextLocale: AppLocale) => {
    const normalizedLocale = normalizeLocale(nextLocale);
    if (normalizedLocale === locale) {
      return;
    }
    setIsSwitchingLocale(true);
    try {
      const settings = await appClient.setSettings({ locale: normalizedLocale });
      setAppSettings(settings);
      toast.success(translate(normalizedLocale, "界面语言已切换"));
    } catch (error: unknown) {
      const message = getAppErrorMessage(error);
      if (message.includes("permission_denied")) {
        setAppSettings({ locale: normalizedLocale });
        toast.success(translate(normalizedLocale, "界面语言已切换"));
        return;
      }
      toast.error(`${translate(locale, "语言切换失败")}: ${message}`);
    } finally {
      setIsSwitchingLocale(false);
    }
  }, [locale, setAppSettings]);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      localeOptions,
      isSwitchingLocale,
      setLocale,
      t,
    }),
    [isSwitchingLocale, locale, localeOptions, setLocale, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    return {
      locale: DEFAULT_LOCALE,
      localeOptions: SUPPORTED_LOCALES.slice(),
      isSwitchingLocale: false,
      setLocale: async () => undefined,
      t: (message: string, values?: TranslationValues) =>
        translate(DEFAULT_LOCALE, message, values),
    };
  }
  return context;
}

export function getLocaleLabel(locale: AppLocale): string {
  return LOCALE_LABELS[locale];
}
