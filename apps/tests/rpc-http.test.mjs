import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";
import ts from "../node_modules/typescript/lib/typescript.js";

const appsRoot = path.resolve(import.meta.dirname, "..");
const rpcHttpSourcePath = path.join(
  appsRoot,
  "src",
  "lib",
  "api",
  "rpc-http.ts"
);
const transportErrorsSourcePath = path.join(
  appsRoot,
  "src",
  "lib",
  "api",
  "transport-errors.ts"
);

async function loadRpcHttpModule() {
  const [rpcHttpSource, transportErrorsSource] = await Promise.all([
    fs.readFile(rpcHttpSourcePath, "utf8"),
    fs.readFile(transportErrorsSourcePath, "utf8"),
  ]);

  const rpcHttpCompiled = ts.transpileModule(rpcHttpSource, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: rpcHttpSourcePath,
  });
  const transportErrorsCompiled = ts.transpileModule(transportErrorsSource, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: transportErrorsSourcePath,
  });

  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "codexmanager-rpc-http-"));
  const rpcHttpFile = path.join(tempDir, "rpc-http.mjs");
  const transportErrorsFile = path.join(tempDir, "transport-errors.mjs");

  await fs.writeFile(
    rpcHttpFile,
    rpcHttpCompiled.outputText.replace(
      /from "\.\/transport-errors"/g,
      'from "./transport-errors.mjs"'
    ),
    "utf8"
  );
  await fs.writeFile(transportErrorsFile, transportErrorsCompiled.outputText, "utf8");
  return import(pathToFileURL(rpcHttpFile).href);
}

const rpcHttp = await loadRpcHttpModule();

test("buildJsonRpcRequestBody 生成标准 JSON-RPC envelope", () => {
  const payload = JSON.parse(
    rpcHttp.buildJsonRpcRequestBody("demo/method", { foo: "bar" })
  );

  assert.equal(payload.jsonrpc, "2.0");
  assert.equal(payload.method, "demo/method");
  assert.deepEqual(payload.params, { foo: "bar" });
  assert.equal(typeof payload.id, "number");
});

test("postJsonRpc 统一通过 fetcher 发送并解包结果", async () => {
  const calls = [];
  const result = await rpcHttp.postJsonRpc(
    async (url, init, options) => {
      calls.push({ url, init, options });
      return {
        ok: true,
        status: 200,
        async json() {
          return { result: { ok: true, value: 7 } };
        },
      };
    },
    "/api/rpc",
    "demo/method",
    { foo: "bar" },
    { retries: 0 }
  );

  assert.equal(result.value, 7);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, "/api/rpc");
  assert.equal(calls[0].options.retries, 0);
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.headers["Content-Type"], "application/json");
});

test("postJsonRpc 对非 2xx 响应会带出服务端错误详情与链路标识", async () => {
  await assert.rejects(
    () =>
      rpcHttp.postJsonRpc(
        async () => ({
          ok: false,
          status: 503,
          headers: {
            get(name) {
              if (name === "X-CodexManager-Error-Code") return "upstream_timeout";
              if (name === "X-CodexManager-Trace-Id") return "trace-rpc-503";
              return null;
            },
          },
          async text() {
            return JSON.stringify({
              error: {
                message: "上游请求超时",
              },
            });
          },
        }),
        "/api/rpc",
        "demo/method"
      ),
    /HTTP 503.*上游请求超时.*upstream_timeout.*trace-rpc-503/
  );
});

test("postJsonRpc 遇到 Web 鉴权失效会跳转登录页", async () => {
  const originalWindow = globalThis.window;
  const redirects = [];
  globalThis.window = {
    location: {
      pathname: "/settings",
      search: "?tab=general",
      replace(url) {
        redirects.push(url);
      },
    },
  };

  try {
    await assert.rejects(
      () =>
        rpcHttp.postJsonRpc(
          async () => ({
            ok: false,
            status: 401,
            headers: { get: () => null },
            async text() {
              return JSON.stringify({ error: "web_auth_required" });
            },
          }),
          "/api/rpc",
          "app/settings/get"
        ),
      /HTTP 401/
    );
  } finally {
    if (originalWindow === undefined) {
      delete globalThis.window;
    } else {
      globalThis.window = originalWindow;
    }
  }

  assert.deepEqual(redirects, [
    "/__login?next=%2Fsettings%3Ftab%3Dgeneral",
  ]);
});
