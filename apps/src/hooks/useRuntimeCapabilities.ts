"use client";

import { useMemo, useSyncExternalStore } from "react";
import { isTauriRuntime } from "@/lib/api/transport";
import {
  resolveRuntimeCapabilityView,
  type RuntimeCapabilityView,
} from "@/lib/runtime/runtime-capabilities";
import { useAppStore } from "@/lib/store/useAppStore";

/**
 * 函数 `useRuntimeCapabilities`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * 无
 *
 * # 返回
 * 返回函数执行结果
 */
export function useRuntimeCapabilities(): RuntimeCapabilityView {
  const runtimeCapabilities = useAppStore((state) => state.runtimeCapabilities);
  const isMounted = useSyncExternalStore(
    () => () => undefined,
    () => true,
    () => false,
  );

  return useMemo(() => {
    // 中文注释：首屏先保持与 SSR 一致，等客户端挂载后再启用 Tauri 运行时探测，避免 hydration mismatch。
    return resolveRuntimeCapabilityView(
      runtimeCapabilities,
      isMounted && isTauriRuntime()
    );
  }, [isMounted, runtimeCapabilities]);
}
