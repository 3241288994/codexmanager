"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { useTheme } from "next-themes";
import { Button } from "@/components/ui/button";
import { appClient } from "@/lib/api/app-client";
import { serviceClient } from "@/lib/api/service-client";
import { loadRuntimeCapabilities } from "@/lib/api/transport";
import { applyAppearancePreset } from "@/lib/appearance";
import { useAppStore } from "@/lib/store/useAppStore";
import { formatServiceError, isExpectedInitializeResult, normalizeServiceAddr } from "@/lib/utils/service";
import { withTimeout } from "@/lib/utils/timeout";

const DEFAULT_SERVICE_ADDR = "localhost:48760";
const INITIALIZE_TIMEOUT_MS = 15_000;

export function AppBootstrap({ children }: { children: React.ReactNode }) {
  const { setAppSettings, setRuntimeCapabilities, setServiceStatus } = useAppStore();
  const { setTheme } = useTheme();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  const initialize = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const runtime = await withTimeout(
        loadRuntimeCapabilities(true),
        INITIALIZE_TIMEOUT_MS,
        "运行环境检测超时",
      );
      setRuntimeCapabilities(runtime);
      if (runtime.mode === "unsupported-web") {
        throw new Error(runtime.unsupportedReason || "请通过 codexmanager-web 访问此页面");
      }

      const settings = await withTimeout(
        appClient.getSettings(),
        INITIALIZE_TIMEOUT_MS,
        "读取控制台设置超时",
      );
      setAppSettings(settings);
      setTheme(settings.theme);
      applyAppearancePreset(settings.appearancePreset);
      document.body.classList.toggle("low-transparency", settings.lowTransparency);

      const addr = normalizeServiceAddr(settings.serviceAddr || DEFAULT_SERVICE_ADDR);
      let result;
      try {
        result = await withTimeout(serviceClient.initialize(addr), INITIALIZE_TIMEOUT_MS, "服务连接超时");
      } catch (connectionError) {
        if (runtime.mode !== "desktop-tauri") throw connectionError;
        await withTimeout(serviceClient.start(addr), INITIALIZE_TIMEOUT_MS, "服务启动超时");
        result = await withTimeout(serviceClient.initialize(addr), INITIALIZE_TIMEOUT_MS, "服务连接超时");
      }
      if (!isExpectedInitializeResult(result)) throw new Error("目标端口不是兼容的 Codex 控制台服务");
      setServiceStatus({ addr, connected: true, version: "" });
    } catch (cause) {
      setServiceStatus({ connected: false, version: "" });
      setError(formatServiceError(cause));
    } finally {
      setLoading(false);
    }
  }, [setAppSettings, setRuntimeCapabilities, setServiceStatus, setTheme]);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void initialize();
  }, [initialize]);

  return (
    <>
      {children}
      {loading || error ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 p-6 backdrop-blur">
          <div className="flex w-full max-w-md flex-col items-center gap-5 rounded-2xl border bg-card p-8 text-center shadow-xl">
            {loading ? (
              <>
                <div className="size-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                <div><h2 className="text-xl font-semibold">正在连接控制台服务</h2><p className="mt-2 text-sm text-muted-foreground">读取账号、额度与 Codex 会话索引。</p></div>
              </>
            ) : (
              <>
                <div className="flex size-12 items-center justify-center rounded-full bg-destructive/10"><AlertCircle className="size-6 text-destructive" /></div>
                <div><h2 className="text-xl font-semibold">无法连接控制台服务</h2><p className="mt-2 break-all rounded-lg bg-muted p-3 font-mono text-xs text-muted-foreground">{error}</p></div>
                <Button onClick={() => void initialize()}><RefreshCw />重新连接</Button>
              </>
            )}
          </div>
        </div>
      ) : null}
    </>
  );
}
