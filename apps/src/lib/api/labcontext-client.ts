import { invoke, withAddr } from "./transport";
import type {
  LabContextLocation,
  LabContextOverview,
  LabContextWorkspaceTarget,
  ResearchMapBundle,
  ResearchMapLayout,
  ResearchMapPatch,
} from "@/types/labcontext";

function call<T>(
  location: LabContextLocation,
  serverCommand: string,
  localOperation: string,
  params: Record<string, unknown> = {},
): Promise<T> {
  if (location === "local") {
    return invoke<T>("app_labcontext_local_call", { operation: localOperation, params });
  }
  return invoke<T>(serverCommand, withAddr(params));
}

function targetParams(target: LabContextWorkspaceTarget): Record<string, unknown> {
  return { workspaceId: target.workspaceId };
}

export const labContextClient = {
  async overview(location: LabContextLocation = "server"): Promise<LabContextOverview> {
    const overview = await call<LabContextOverview>(location, "service_labcontext_overview", "overview");
    return {
      ...overview,
      workspaces: overview.workspaces.map((workspace) => ({ ...workspace, location })),
    };
  },
  setDefaultWorkspace(target: LabContextWorkspaceTarget): Promise<{ ok: boolean; defaultWorkspaceId: string }> {
    return call(target.location, "service_labcontext_set_default_workspace", "setDefaultWorkspace", targetParams(target));
  },
  refreshWorkspace(target: LabContextWorkspaceTarget): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_refresh_workspace", "refreshWorkspace", targetParams(target));
  },
  testTool(tool: string, target?: LabContextWorkspaceTarget): Promise<{ tool: string; input: unknown; result: unknown; responseBytes: number; elapsedMs: number; testedAt: string }> {
    const location = target?.location || "server";
    return call(location, "service_labcontext_test_tool", "testTool", { tool, workspaceId: target?.workspaceId || null });
  },
  setToolPolicy(location: LabContextLocation, profile: string, disabledTools: string[]): Promise<Record<string, unknown>> {
    return call(location, "service_labcontext_set_tool_policy", "setToolPolicy", { profile, disabledTools });
  },
  upsertWorkspace(payload: {
    name: string;
    root: string;
    location: LabContextLocation;
  }): Promise<{ ok: boolean; workspaceId: string; overviewGeneration?: WorkspaceOverviewGeneration }> {
    if (payload.location === "local") {
      return invoke("app_labcontext_local_upsert_workspace", { name: payload.name, root: payload.root });
    }
    return invoke("service_labcontext_upsert_workspace", withAddr({ name: payload.name, root: payload.root }));
  },
  pickLocalWorkspaceDirectory(): Promise<{ canceled: boolean; path: string | null }> {
    return invoke("app_labcontext_pick_local_workspace_directory");
  },
  deleteWorkspace(target: LabContextWorkspaceTarget): Promise<{ ok: boolean; workspaceId: string; projectFilesDeleted: boolean }> {
    return call(target.location, "service_labcontext_delete_workspace", "deleteWorkspace", targetParams(target));
  },
  setWorkspaceOverview(target: LabContextWorkspaceTarget, overview: string): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_set_workspace_overview", "setWorkspaceOverview", { ...targetParams(target), overview });
  },
  generateWorkspaceOverview(target: LabContextWorkspaceTarget, refresh = false): Promise<WorkspaceOverviewGeneration> {
    return call(target.location, "service_labcontext_generate_workspace_overview", "generateWorkspaceOverview", { ...targetParams(target), refresh });
  },
  setWorkerConfig(location: LabContextLocation, model: string, reasoningEffort: string): Promise<Record<string, unknown>> {
    return call(location, "service_labcontext_set_worker_config", "setWorkerConfig", { model, reasoningEffort });
  },
  getResearchMap(target: LabContextWorkspaceTarget): Promise<ResearchMapBundle> {
    return call(target.location, "service_labcontext_get_research_map", "getResearchMap", targetParams(target));
  },
  initializeResearchMap(target: LabContextWorkspaceTarget): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_initialize_research_map", "initializeResearchMap", targetParams(target));
  },
  saveResearchMapLayout(target: LabContextWorkspaceTarget, layout: Pick<ResearchMapLayout, "nodes" | "viewport">): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_save_research_map_layout", "saveResearchMapLayout", { ...targetParams(target), layout });
  },
  applyResearchMapPatch(target: LabContextWorkspaceTarget, patch: ResearchMapPatch): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_apply_research_map_patch", "applyResearchMapPatch", { ...targetParams(target), patch });
  },
  reviewResearchMap(target: LabContextWorkspaceTarget, preferQueue = true): Promise<{ status: string; proposalId?: string; session?: { sessionId: string; updatedAt: string; workspaceMatch: string } | null; message: string }> {
    return call(target.location, "service_labcontext_review_research_map", "reviewResearchMap", { ...targetParams(target), preferQueue });
  },
  researchMapProposalAction(target: LabContextWorkspaceTarget, proposalId: string, action: "apply" | "reject"): Promise<Record<string, unknown>> {
    return call(target.location, "service_labcontext_research_map_proposal_action", "researchMapProposalAction", { ...targetParams(target), proposalId, action });
  },
};

export interface WorkspaceOverviewGeneration {
  workspaceId: string;
  jobId: string;
  status: "running" | "ready" | "completed" | "failed" | "not_found";
  progress?: string;
  overview?: string;
  error?: string;
}
