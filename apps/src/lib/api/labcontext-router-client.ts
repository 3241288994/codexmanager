import { fetchWithRetry } from "@/lib/utils/request";

const DEFAULT_ROUTER_URL = "http://127.0.0.1:1460";
const LOCAL_PROVIDER_ID = process.env.NEXT_PUBLIC_LABCONTEXT_LOCAL_PROVIDER_ID || "local";

export type LabContextRouterProvider = {
  id: string;
  label: string;
  mcpUrl: string;
  adminEnabled: boolean;
  status: "configured" | "ready" | "unavailable";
  adminStatus: "disabled" | "configured" | "ready" | "unavailable";
  error?: string;
  adminError?: string;
  serverName?: string | null;
  serverVersion?: string | null;
  minimumVersion?: string;
  protocolVersion?: string | null;
  toolCount?: number;
  workspaceCount?: number;
  missingTools?: string[];
  latencyMs?: number;
};

export type LabContextRuntimeComponent = {
  id: string;
  label: string;
  pid?: number;
  status: "starting" | "running" | "retrying" | "exited";
  exitCode?: number | null;
  retryInSeconds?: number;
  restartAttempts?: number;
};

export type LabContextConnectionStatus = {
  schemaVersion: number;
  checkedAt: string;
  overall: "ready" | "degraded" | "unavailable";
  router: {
    status: "ready";
    name: string;
    version: string;
    pid: number;
  };
  launcher: {
    status: "running" | "stale" | "unknown";
    detail?: string;
    launcherPid?: number;
    startedAt?: string;
    updatedAt?: string;
    phase?: string;
    components?: LabContextRuntimeComponent[];
  };
  providers: LabContextRouterProvider[];
  tunnel: {
    enabled: boolean;
    status: "ready" | "unavailable" | "disabled";
    detail: string;
    latencyMs?: number;
  };
};

function routerUrl(path: string): string {
  const base = process.env.NEXT_PUBLIC_LABCONTEXT_ROUTER_URL || DEFAULT_ROUTER_URL;
  return `${base.replace(/\/$/, "")}${path}`;
}

async function responseJson<T>(response: Response): Promise<T> {
  const value = await response.json().catch(() => ({})) as { error?: string };
  if (!response.ok) {
    throw new Error(value.error || `LabContext Router 请求失败（HTTP ${response.status}）`);
  }
  return value as T;
}

export async function probeLabContextRouter(): Promise<{
  status: LabContextConnectionStatus;
  providers: LabContextRouterProvider[];
  localAvailable: boolean;
}> {
  const response = await fetchWithRetry(routerUrl("/api/status"), {
    headers: { Accept: "application/json" },
  }, { timeoutMs: 3_000, retries: 0 });
  const status = await responseJson<LabContextConnectionStatus>(response);
  const providers = status.providers || [];
  const local = providers.find((provider) => provider.id === LOCAL_PROVIDER_ID);
  return {
    status,
    providers,
    localAvailable: Boolean(
      local?.adminEnabled
      && local.status === "ready"
      && local.adminStatus === "ready"
    ),
  };
}

export async function callLocalLabContextRouter<T>(
  operation: string,
  params: Record<string, unknown> = {},
): Promise<T> {
  const response = await fetchWithRetry(
    routerUrl(`/api/admin/${encodeURIComponent(LOCAL_PROVIDER_ID)}/call`),
    {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ operation, params }),
    },
    { timeoutMs: 30_000, retries: 0 },
  );
  const payload = await responseJson<{ result: T }>(response);
  return payload.result;
}
