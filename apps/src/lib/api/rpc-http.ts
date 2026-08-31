import type { RequestOptions } from "../utils/request";
import { getAppErrorMessage, unwrapRpcPayload } from "./transport-errors";

export type JsonRpcFetcher = (
  url: string,
  init?: RequestInit,
  options?: RequestOptions
) => Promise<Response>;

export function buildJsonRpcRequestBody(
  method: string,
  params: Record<string, unknown> = {}
): string {
  return JSON.stringify({
    jsonrpc: "2.0",
    id: Date.now(),
    method,
    params,
  });
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

async function readRpcHttpErrorPayload(response: Response): Promise<unknown> {
  if (typeof response.text === "function") {
    try {
      const raw = await response.text();
      const trimmed = raw.trim();
      if (!trimmed) {
        return null;
      }
      try {
        return JSON.parse(trimmed) as unknown;
      } catch {
        return trimmed;
      }
    } catch {
      // Ignore text parsing failure and fall through to json().
    }
  }

  if (typeof response.json === "function") {
    try {
      return (await response.json()) as unknown;
    } catch {
      return null;
    }
  }

  return null;
}

function buildRpcHttpErrorMessage(
  status: number,
  payload: unknown,
  headers?: Pick<Headers, "get">
): string {
  const detail = getAppErrorMessage(payload, "").trim();
  const errorCode =
    headers?.get("X-CodexManager-Error-Code")?.trim() ||
    asRecord(payload)?.errorCode?.toString().trim() ||
    "";
  const traceId = headers?.get("X-CodexManager-Trace-Id")?.trim() || "";

  let message = detail
    ? `RPC 请求失败（HTTP ${status}）：${detail}`
    : `RPC 请求失败（HTTP ${status}）`;
  const meta = [
    errorCode ? `code=${errorCode}` : "",
    traceId ? `trace=${traceId}` : "",
  ].filter(Boolean);
  if (meta.length) {
    message += ` [${meta.join(" ")}]`;
  }
  return message;
}

function isWebAuthRequiredPayload(payload: unknown): boolean {
  const record = asRecord(payload);
  return record?.error === "web_auth_required";
}

function redirectToWebLoginIfBrowser(): void {
  if (typeof window === "undefined") {
    return;
  }
  const currentPath =
    `${window.location.pathname || "/"}${window.location.search || ""}` || "/";
  const loginUrl = `/__login?next=${encodeURIComponent(currentPath)}`;
  window.location.replace(loginUrl);
}

export async function postJsonRpc<T>(
  fetcher: JsonRpcFetcher,
  url: string,
  rpcMethod: string,
  params: Record<string, unknown> = {},
  options: RequestOptions = {}
): Promise<T> {
  const response = await fetcher(
    url,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: buildJsonRpcRequestBody(rpcMethod, params),
    },
    options
  );

  if (!response.ok) {
    const payload = await readRpcHttpErrorPayload(response);
    if (response.status === 401 && isWebAuthRequiredPayload(payload)) {
      redirectToWebLoginIfBrowser();
    }
    throw new Error(
      buildRpcHttpErrorMessage(response.status, payload, response.headers)
    );
  }

  return unwrapRpcPayload<T>((await response.json()) as unknown);
}
