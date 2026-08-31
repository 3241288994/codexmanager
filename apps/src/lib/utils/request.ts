export interface RequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
  retries?: number;
  retryDelayMs?: number;
  maxRetryDelayMs?: number;
  shouldRetry?: (error: unknown) => boolean;
  shouldRetryStatus?: (status: number) => boolean;
}

/**
 * 函数 `fetchWithRetry`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - url: 参数 url
 * - init?: 参数 init?
 * - options: 参数 options
 *
 * # 返回
 * 返回函数执行结果
 */
export async function fetchWithRetry(
  url: string,
  init?: RequestInit,
  options: RequestOptions = {}
): Promise<Response> {
  const {
    timeoutMs = 10000,
    retries = 3,
    retryDelayMs = 200,
    maxRetryDelayMs = 3000,
    shouldRetryStatus = (status) => status >= 500 || status === 429,
  } = options;

  let lastError: unknown;
  for (let i = 0; i <= retries; i++) {
    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), timeoutMs);
    if (options.signal) {
      options.signal.addEventListener("abort", () => controller.abort());
    }

    try {
      const response = await fetch(url, {
        ...init,
        signal: controller.signal,
      });
      clearTimeout(id);

      if (response.ok || !shouldRetryStatus(response.status) || i === retries) {
        return response;
      }
    } catch (err: unknown) {
      clearTimeout(id);
      lastError = err;
      if (err instanceof Error && err.name === "AbortError" && !options.signal?.aborted) {
        // Timeout retry
      } else if (i === retries) {
        throw err;
      }
    }

    const delay = Math.min(retryDelayMs * Math.pow(2, i), maxRetryDelayMs);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
  throw lastError || new Error("Fetch failed after retries");
}

/**
 * 函数 `runWithControl`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - fn: 参数 fn
 * - options: 参数 options
 *
 * # 返回
 * 返回函数执行结果
 */
export async function runWithControl<T>(
  fn: () => Promise<T>,
  options: RequestOptions = {}
): Promise<T> {
  const {
    retries = 0,
    retryDelayMs = 200,
    maxRetryDelayMs = 3000,
    shouldRetry = () => true,
  } = options;

  let lastError: unknown;
  for (let i = 0; i <= retries; i++) {
    try {
      return await fn();
    } catch (err: unknown) {
      lastError = err;
      if (i === retries || !shouldRetry(err)) {
        throw err;
      }
    }
    const delay = Math.min(retryDelayMs * Math.pow(2, i), maxRetryDelayMs);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
  throw lastError;
}
