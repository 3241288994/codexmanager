import { isTauri as tauriIsTauri } from "@tauri-apps/api/core";
import { fetchWithRetry } from "../utils/request";
import {
  buildDesktopRuntimeCapabilities,
  buildUnsupportedWebCapabilities,
  buildWebGatewayRuntimeCapabilities,
  DEFAULT_UNSUPPORTED_WEB_REASON,
  normalizeRpcBaseUrl,
  normalizeRuntimeCapabilities,
} from "../runtime/runtime-capabilities";
import { RuntimeCapabilities } from "../../types";

const DEFAULT_WEB_RPC_BASE_URL = "/api/rpc";
const DEFAULT_RUNTIME_PROBE_URL = "/api/runtime";
const CONFIGURED_WEB_RPC_BASE_URL = normalizeRpcBaseUrl(
  process.env.NEXT_PUBLIC_CODEXMANAGER_RPC_BASE_URL
);

let runtimeCapabilitiesCache: RuntimeCapabilities | null = null;
let runtimeCapabilitiesPromise: Promise<RuntimeCapabilities> | null = null;

function runtimeProbeUrl(): string {
  if (typeof window === "undefined") {
    return DEFAULT_RUNTIME_PROBE_URL;
  }
  const probeUrl = new URL(DEFAULT_RUNTIME_PROBE_URL, window.location.origin);
  probeUrl.searchParams.set("_", String(Date.now()));
  return probeUrl.toString();
}

async function probeRuntimeCapabilities(): Promise<RuntimeCapabilities | null> {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const response = await fetchWithRetry(
      runtimeProbeUrl(),
      {
        method: "GET",
        cache: "no-store",
        headers: {
          Accept: "application/json",
        },
      },
      {
        timeoutMs: 2500,
        retries: 2,
        retryDelayMs: 250,
        shouldRetryStatus: () => false,
      }
    );
    if (!response.ok) {
      return null;
    }
    return normalizeRuntimeCapabilities(
      await response.json(),
      CONFIGURED_WEB_RPC_BASE_URL || DEFAULT_WEB_RPC_BASE_URL
    );
  } catch {
    return null;
  }
}

function cacheRuntimeCapabilities(
  runtimeCapabilities: RuntimeCapabilities
): RuntimeCapabilities {
  runtimeCapabilitiesCache = runtimeCapabilities;
  return runtimeCapabilities;
}

export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const runtime = globalThis as typeof globalThis & {
    __TAURI__?: unknown;
    __TAURI_INTERNALS__?: { invoke?: unknown };
  };

  return (
    tauriIsTauri() ||
    Boolean(runtime.__TAURI_INTERNALS__?.invoke) ||
    Boolean(runtime.__TAURI__)
  );
}

export function getCachedRuntimeCapabilities(): RuntimeCapabilities | null {
  if (isTauriRuntime()) {
    return runtimeCapabilitiesCache ?? buildDesktopRuntimeCapabilities();
  }
  return runtimeCapabilitiesCache;
}

export async function loadRuntimeCapabilities(
  force = false
): Promise<RuntimeCapabilities> {
  if (isTauriRuntime()) {
    return cacheRuntimeCapabilities(buildDesktopRuntimeCapabilities());
  }
  if (!force && runtimeCapabilitiesCache) {
    return runtimeCapabilitiesCache;
  }
  if (!force && runtimeCapabilitiesPromise) {
    return runtimeCapabilitiesPromise;
  }

  runtimeCapabilitiesPromise = (async () => {
    const probedRuntime = await probeRuntimeCapabilities();
    if (probedRuntime) {
      return cacheRuntimeCapabilities(probedRuntime);
    }
    if (CONFIGURED_WEB_RPC_BASE_URL) {
      return cacheRuntimeCapabilities(
        buildWebGatewayRuntimeCapabilities(CONFIGURED_WEB_RPC_BASE_URL)
      );
    }
    return cacheRuntimeCapabilities(
      buildUnsupportedWebCapabilities(
        DEFAULT_UNSUPPORTED_WEB_REASON,
        DEFAULT_WEB_RPC_BASE_URL
      )
    );
  })();

  try {
    return await runtimeCapabilitiesPromise;
  } finally {
    runtimeCapabilitiesPromise = null;
  }
}
