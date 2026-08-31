import { create } from "zustand";
import type { AppSettings, RuntimeCapabilities, ServiceStatus } from "../../types";

interface AppState {
  serviceStatus: ServiceStatus;
  appSettings: AppSettings;
  runtimeCapabilities: RuntimeCapabilities | null;
  isSidebarOpen: boolean;
  setServiceStatus: (status: Partial<ServiceStatus>) => void;
  setAppSettings: (settings: Partial<AppSettings>) => void;
  setRuntimeCapabilities: (capabilities: RuntimeCapabilities | null) => void;
  toggleSidebar: () => void;
}

const initialSettings = {
  locale: "zh-CN",
  localeOptions: ["zh-CN", "en", "ru", "ko"],
  serviceAddr: "localhost:48760",
  webAuthMode: "none",
  webAccessPasswordConfigured: false,
  theme: "tech",
  appearancePreset: "classic",
  lowTransparency: false,
} as AppSettings;

export const useAppStore = create<AppState>((set) => ({
  serviceStatus: {
    connected: false,
    version: "",
    uptime: 0,
    addr: "localhost:48760",
  },
  appSettings: initialSettings,
  runtimeCapabilities: null,
  isSidebarOpen: true,
  setServiceStatus: (status) => set((state) => ({ serviceStatus: { ...state.serviceStatus, ...status } })),
  setAppSettings: (settings) => set((state) => ({ appSettings: { ...state.appSettings, ...settings } })),
  setRuntimeCapabilities: (runtimeCapabilities) => set({ runtimeCapabilities }),
  toggleSidebar: () => set((state) => ({ isSidebarOpen: !state.isSidebarOpen })),
}));
