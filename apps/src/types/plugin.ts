export interface PluginCatalogTask {
  id: string;
  name: string;
  description: string | null;
  entrypoint: string;
  scheduleKind: string;
  intervalSeconds: number | null;
  enabled: boolean;
}

export interface PluginCatalogEntry {
  id: string;
  name: string;
  version: string;
  description: string | null;
  author: string | null;
  homepageUrl: string | null;
  scriptUrl: string | null;
  scriptBody: string | null;
  permissions: string[];
  tasks: PluginCatalogTask[];
  manifestVersion: string;
  category: string | null;
  runtimeKind: string;
  tags: string[];
  sourceUrl: string | null;
}

export interface InstalledPluginSummary {
  pluginId: string;
  sourceUrl: string | null;
  name: string;
  version: string;
  description: string | null;
  author: string | null;
  homepageUrl: string | null;
  scriptUrl: string | null;
  permissions: string[];
  status: string;
  installedAt: number;
  updatedAt: number;
  lastRunAt: number | null;
  lastError: string | null;
  taskCount: number;
  enabledTaskCount: number;
  manifestVersion: string;
  category: string | null;
  runtimeKind: string;
  tags: string[];
}

export interface PluginTaskSummary {
  id: string;
  pluginId: string;
  pluginName: string;
  name: string;
  description: string | null;
  entrypoint: string;
  scheduleKind: string;
  intervalSeconds: number | null;
  enabled: boolean;
  nextRunAt: number | null;
  lastRunAt: number | null;
  lastStatus: string | null;
  lastError: string | null;
}

export interface PluginRunLogSummary {
  id: number;
  pluginId: string;
  pluginName: string | null;
  taskId: string | null;
  taskName: string | null;
  runType: string;
  status: string;
  startedAt: number;
  finishedAt: number | null;
  durationMs: number | null;
  output: unknown | null;
  error: string | null;
}

export interface PluginCatalogResult {
  sourceUrl: string;
  items: PluginCatalogEntry[];
}
