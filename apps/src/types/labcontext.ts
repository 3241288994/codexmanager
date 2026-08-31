export type LabContextHealthState = "healthy" | "degraded" | "unknown" | "down";

export interface LabContextAssetCoverage {
  assetId: string;
  kind: string;
  authority: string;
  indexContent: string;
  include: string[];
  exclude: string[];
  fileCount: number;
  totalBytes: number;
  truncated: boolean;
  newestModifiedAt: string | null;
  status: "ready" | "empty" | "error";
  error: string | null;
}

export interface LabContextWorkspace {
  workspaceId: string;
  name: string;
  root: string;
  aliases: string[];
  adapters: string[];
  isDefault: boolean;
  status: string;
  git: { branch: string | null; commit: string | null; headState: string; dirty: boolean; changedPathCount: number };
  description: string;
  overviewSource: "reviewed" | "codex" | "automatic";
  context: {
    path: string;
    status: string;
    generation: { status: string; jobId?: string; model?: string; reasoningEffort?: string; error?: string } | null;
  };
  assets: LabContextAssetCoverage[];
  readableAssets: Array<{
    kind: string;
    label: string;
    meaning: string;
    fileCount: number;
    status: string;
    newestModifiedAt: string | null;
  }>;
  coverage: { fileCount: number; totalBytes: number; emptyAssets: number };
  researchMap: ResearchMapFocusCapsule & { pendingProposals: number; error?: string };
}

export type ResearchMapNodeType = "core_idea" | "claim" | "branch" | "current_target" | "experiment" | "evidence" | "decision" | "risk";

export interface ResearchMapNode {
  id: string;
  type: ResearchMapNodeType;
  title: string;
  summary: string;
  status: string;
  authority: string;
  origin: string;
  lockedFields: string[];
  sourceRefs: string[];
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export interface ResearchMapEdge {
  id: string;
  source: string;
  target: string;
  relation: string;
  label: string;
  origin: string;
  sourceRefs: string[];
  createdAt: string;
}

export interface ResearchMapDocument {
  schemaVersion: number;
  workspaceId: string;
  revision: number;
  reviewState: string;
  currentFocusNodeId: string | null;
  nodes: ResearchMapNode[];
  edges: ResearchMapEdge[];
  createdAt: string;
  updatedAt: string;
  updatedBy: string;
}

export interface ResearchMapLayout {
  schemaVersion: number;
  workspaceId: string;
  nodes: Array<{ id: string; x: number; y: number; collapsed: boolean }>;
  viewport: { x: number; y: number; zoom: number };
  updatedAt: string | null;
  updatedBy: string | null;
}

export interface ResearchMapFocusCapsule {
  status: string;
  mapRevision: number;
  reviewState: string;
  coreIdea: { id: string; title: string } | null;
  activeClaim: { id: string; title: string; status: string } | null;
  currentTarget: { id: string; title: string; status: string } | null;
  nextExperiment: { id: string; title: string; status: string } | null;
  blockers: Array<{ id: string; title: string }>;
  relatedNodes: Array<{ id: string; type: string; title: string; status: string }>;
  counts: { nodes: number; edges: number; activeBranches: number; openRisks: number };
  updatedAt: string;
}

export interface ResearchMapPatchOperation {
  op: string;
  [key: string]: unknown;
}

export interface ResearchMapPatch {
  baseRevision: number;
  summary: string;
  operations: ResearchMapPatchOperation[];
}

export interface ResearchMapProposal {
  proposalId: string;
  workspaceId: string;
  baseRevision: number;
  sourceSessionId: string | null;
  sourceKind: string;
  status: string;
  patch: ResearchMapPatch | null;
  summary: string | null;
  error: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ResearchMapEvent {
  eventId: string;
  eventType: string;
  workspaceId: string;
  revision: number;
  actor: string;
  summary: string;
  timestamp: string;
}

export interface ResearchMapBundle {
  workspaceId: string;
  researchMap: ResearchMapDocument;
  layout: ResearchMapLayout;
  focusCapsule: ResearchMapFocusCapsule;
  proposals: ResearchMapProposal[];
  events: ResearchMapEvent[];
}

export interface LabContextTool {
  name: string;
  description: string;
  latencyClass: "instant" | "indexed" | "codex";
  dependencies: string[];
  enabled: boolean;
  readOnly: boolean;
  computeCost: string;
}

export interface LabContextJob {
  jobId: string;
  workspaceId: string;
  status: string;
  progress: string | null;
  startedAt: string | null;
  updatedAt: string | null;
  generatedAt: string | null;
  errorType: string | null;
  diagnostics: Record<string, unknown> | null;
}

export interface LabContextAuditRecord {
  timestamp: string;
  event: string;
  workspaceId?: string;
  status?: string;
  jobId?: string;
  resultCount?: number;
  experimentCount?: number;
  [key: string]: unknown;
}

export interface LabContextOverview {
  version: string;
  generatedAt: string;
  defaultWorkspaceId: string;
  configPath: string;
  health: {
    overall: LabContextHealthState;
    generatedAt: string;
    checks: Array<{ id: string; label: string; status: LabContextHealthState; detail: string }>;
  };
  workspaces: LabContextWorkspace[];
  workerConfig: {
    model: string;
    reasoningEffort: string;
    availableModels: string[];
    availableEfforts: Record<string, string[]>;
    appliesTo: string;
  };
  toolPolicy: { profile: string; tools: LabContextTool[] };
  activity: { records: LabContextAuditRecord[]; totalReturned: number };
  jobs: { jobs: LabContextJob[]; statusCounts: Record<string, number> };
}
