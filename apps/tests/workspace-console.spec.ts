import { expect, test, type Page } from "@playwright/test";

const now = new Date("2026-08-31T12:00:00.000Z").toISOString();

const settings = {
  locale: "zh-CN",
  localeOptions: ["zh-CN", "en"],
  serviceAddr: "localhost:48760",
  webAuthMode: "none",
  theme: "tech",
  appearancePreset: "classic",
  lowTransparency: false,
};

const labContextOverview = {
  version: "test",
  generatedAt: now,
  defaultWorkspaceId: "workspace-1",
  configPath: "/tmp/labcontext.yaml",
  health: {
    overall: "healthy",
    generatedAt: now,
    checks: [],
  },
  workspaces: [
    {
      workspaceId: "workspace-1",
      name: "Example Research",
      root: "/tmp/example-research",
      aliases: [],
      adapters: [],
      isDefault: true,
      status: "ready",
      git: {
        branch: "main",
        commit: "abc123",
        headState: "clean",
        dirty: false,
        changedPathCount: 0,
      },
      description: "A mocked LabContext workspace.",
      overviewSource: "reviewed",
      context: { path: "/tmp/example-research/.labcontext/context.yaml", status: "ready", generation: null },
      assets: [],
      readableAssets: [],
      coverage: { fileCount: 0, totalBytes: 0, emptyAssets: 0 },
      researchMap: {
        status: "empty",
        mapRevision: 0,
        reviewState: "idle",
        coreIdea: null,
        activeClaim: null,
        currentTarget: null,
        nextExperiment: null,
        blockers: [],
        relatedNodes: [],
        counts: { nodes: 0, edges: 0, activeBranches: 0, openRisks: 0 },
        updatedAt: now,
        pendingProposals: 0,
      },
    },
  ],
  workerConfig: {
    model: "gpt-5.4",
    reasoningEffort: "medium",
    availableModels: ["gpt-5.4"],
    availableEfforts: { "gpt-5.4": ["medium"] },
    appliesTo: "new analysis jobs",
  },
  toolPolicy: { profile: "research", tools: [] },
  activity: { records: [], totalReturned: 0 },
  jobs: { jobs: [], statusCounts: {} },
};

const researchMapBundle = {
  workspaceId: "workspace-1",
  researchMap: {
    schemaVersion: 1,
    workspaceId: "workspace-1",
    revision: 0,
    reviewState: "idle",
    currentFocusNodeId: null,
    nodes: [],
    edges: [],
    createdAt: now,
    updatedAt: now,
    updatedBy: "test",
  },
  layout: {
    schemaVersion: 1,
    workspaceId: "workspace-1",
    nodes: [],
    viewport: { x: 0, y: 0, zoom: 1 },
    updatedAt: null,
    updatedBy: null,
  },
  focusCapsule: {
    status: "empty",
    mapRevision: 0,
    reviewState: "idle",
    coreIdea: null,
    activeClaim: null,
    currentTarget: null,
    nextExperiment: null,
    blockers: [],
    relatedNodes: [],
    counts: { nodes: 0, edges: 0, activeBranches: 0, openRisks: 0 },
    updatedAt: now,
  },
  proposals: [],
  events: [],
};

async function mockConsoleApi(page: Page) {
  const methods: string[] = [];
  await page.route("**/api/runtime**", async (route) => {
    await route.fulfill({
      contentType: "application/json; charset=utf-8",
      body: JSON.stringify({
        mode: "web-gateway",
        rpcBaseUrl: "/api/rpc",
        canManageService: false,
        canSelfUpdate: false,
        canCloseToTray: false,
        canOpenLocalDir: false,
        canUseBrowserFileImport: true,
        canUseBrowserDownloadExport: true,
      }),
    });
  });
  await page.route("**/api/rpc", async (route) => {
    const request = route.request();
    const body = JSON.parse(request.postData() || "{}");
    methods.push(body.method);
    const result = {
      "appSettings/get": settings,
      initialize: {
        userAgent: "codex_cli_rs/0.1.0",
        codexHome: "/tmp/codex",
        platformFamily: "unix",
        platformOs: "linux",
      },
      "account/list": {
        items: [{
          id: "account-1",
          label: "Research account",
          status: "active",
          hasToken: true,
          planType: "plus",
        }],
        total: 1,
      },
      "account/usage/list": {
        items: [{
          accountId: "account-1",
          availabilityStatus: "available",
          usedPercent: 12,
          windowMinutes: 300,
          resetsAt: 1_800_000_000,
          capturedAt: 1_788_000_000,
        }],
      },
      "account/usage/pollingStatus": {
        usagePollingEnabled: true,
        usagePollIntervalSecs: 600,
      },
      "codexProfile/get": {
        codexHome: "/tmp/codex",
        mode: "direct_account",
        selectedAccountId: "account-1",
        actualAccountId: "account-1",
        identityConsistent: true,
      },
      "sessionCatalog/list": {
        codexHome: "/tmp/codex",
        total: 1,
        items: [{
          id: "session-1",
          title: "Session Demo",
          cwd: "/tmp/example-research",
          rolloutPath: "/tmp/rollout.jsonl",
          modelProvider: "openai",
          model: "gpt-5.4",
          reasoningEffort: "medium",
          updatedAtMs: 1_788_000_000_000,
          archived: false,
          rolloutExists: true,
          cwdExists: true,
          visibility: "ready",
          resumeCommand: "codex resume session-1",
        }],
        diagnostics: {
          stateDbAvailable: true,
          checkedCount: 1,
          providerMismatchCount: 0,
          missingRolloutCount: 0,
          missingCwdCount: 0,
          message: "mocked",
        },
      },
      "labcontext/overview": labContextOverview,
      "labcontext/getResearchMap": researchMapBundle,
    }[body.method] ?? {};
    await route.fulfill({
      contentType: "application/json; charset=utf-8",
      body: JSON.stringify({ jsonrpc: "2.0", id: body.id ?? 1, result }),
    });
  });
  return methods;
}

test("the maintained account, session, and LabContext routes load through Web RPC", async ({ page }) => {
  const methods = await mockConsoleApi(page);

  await page.goto("/");
  await expect(page.getByText("账号与额度", { exact: true }).first()).toBeVisible();
  await expect(page.getByTitle("Research account")).toBeVisible();
  await expect(page.getByText("服务已连接", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "会话与恢复" }).click();
  await expect(page.getByText("Session Demo", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("本地会话", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "科研工作区" }).click();
  await expect(page.getByText("Example Research", { exact: true })).toBeVisible();
  await expect(page.getByText("模型可见工具", { exact: true })).toBeVisible();

  for (const method of [
    "appSettings/get",
    "initialize",
    "account/list",
    "account/usage/list",
    "codexProfile/get",
    "sessionCatalog/list",
    "labcontext/overview",
    "labcontext/getResearchMap",
  ]) {
    expect(methods).toContain(method);
  }
});
