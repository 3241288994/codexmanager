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
  providers: LabContextRouterProvider[];
  localAvailable: boolean;
}> {
  const response = await fetchWithRetry(routerUrl("/api/providers"), {
    headers: { Accept: "application/json" },
  }, { timeoutMs: 3_000, retries: 0 });
  const payload = await responseJson<{ providers?: LabContextRouterProvider[] }>(response);
  const providers = payload.providers || [];
  const local = providers.find((provider) => provider.id === LOCAL_PROVIDER_ID);
  return {
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
