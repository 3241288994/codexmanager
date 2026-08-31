import { createServer } from "node:http";
import { existsSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const port = Number.parseInt(process.env.PORT || "3200", 10);
const outDir = resolve(process.cwd(), "out");

if (!existsSync(outDir)) {
  console.error(`静态产物目录不存在: ${outDir}`);
  process.exit(1);
}

const MIME_TYPES = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
};

function isRouteDataRequest(requestUrl) {
  return requestUrl.searchParams.has("_rsc");
}

function resolveFilePath(requestUrl) {
  const pathname = requestUrl.pathname;
  const relativePath = pathname.replace(/^\/+/, "");
  const directPath = join(outDir, relativePath);
  const directoryIndexPath = join(outDir, relativePath, "index.html");
  const directoryRscPath = join(outDir, relativePath, "index.txt");
  const rootRscPath = join(outDir, "index.txt");
  const rootIndexPath = join(outDir, "index.html");

  if (!relativePath) {
    if (isRouteDataRequest(requestUrl) && existsSync(rootRscPath)) {
      return rootRscPath;
    }
    return rootIndexPath;
  }

  if (existsSync(directPath) && statSync(directPath).isFile()) {
    return directPath;
  }

  if (isRouteDataRequest(requestUrl)) {
    if (existsSync(directoryRscPath) && statSync(directoryRscPath).isFile()) {
      return directoryRscPath;
    }
    return null;
  }

  if (existsSync(directoryIndexPath) && statSync(directoryIndexPath).isFile()) {
    return directoryIndexPath;
  }

  return null;
}

const server = createServer(async (request, response) => {
  const requestUrl = new URL(request.url || "/", `http://127.0.0.1:${port}`);
  const filePath = resolveFilePath(requestUrl);

  if (!filePath) {
    console.warn(`[static-server] 404 ${requestUrl.pathname}`);
    response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    response.end("Not Found");
    return;
  }

  try {
    const body = await readFile(filePath);
    const fileExtension = extname(filePath).toLowerCase();
    response.writeHead(200, {
      "Cache-Control": "no-cache",
      "Content-Type": MIME_TYPES[fileExtension] || "application/octet-stream",
    });
    response.end(body);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    response.writeHead(500, { "Content-Type": "text/plain; charset=utf-8" });
    response.end(message);
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`静态测试服务已启动: http://127.0.0.1:${port}`);
});
