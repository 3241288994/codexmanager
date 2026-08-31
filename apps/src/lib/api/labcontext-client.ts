import { invoke, withAddr } from "./transport";
import type { LabContextOverview, ResearchMapBundle, ResearchMapLayout, ResearchMapPatch } from "@/types/labcontext";

export const labContextClient = {
  overview(): Promise<LabContextOverview> {
    return invoke<LabContextOverview>("service_labcontext_overview", withAddr());
  },
  setDefaultWorkspace(workspaceId: string): Promise<{ ok: boolean; defaultWorkspaceId: string }> {
    return invoke("service_labcontext_set_default_workspace", withAddr({ workspaceId }));
  },
  refreshWorkspace(workspaceId: string): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_refresh_workspace", withAddr({ workspaceId }));
  },
  testTool(tool: string, workspaceId?: string): Promise<{ tool: string; input: unknown; result: unknown; responseBytes: number; elapsedMs: number; testedAt: string }> {
    return invoke("service_labcontext_test_tool", withAddr({ tool, workspaceId: workspaceId || null }));
  },
  setToolPolicy(profile: string, disabledTools: string[]): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_set_tool_policy", withAddr({ profile, disabledTools }));
  },
  upsertWorkspace(payload: {
    name: string;
    root: string;
  }): Promise<{ ok: boolean; workspaceId: string; overviewGeneration?: WorkspaceOverviewGeneration }> {
    return invoke("service_labcontext_upsert_workspace", withAddr(payload));
  },
  deleteWorkspace(workspaceId: string): Promise<{ ok: boolean; workspaceId: string; projectFilesDeleted: boolean }> {
    return invoke("service_labcontext_delete_workspace", withAddr({ workspaceId }));
  },
  setWorkspaceOverview(workspaceId: string, overview: string): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_set_workspace_overview", withAddr({ workspaceId, overview }));
  },
  generateWorkspaceOverview(workspaceId: string, refresh = false): Promise<WorkspaceOverviewGeneration> {
    return invoke("service_labcontext_generate_workspace_overview", withAddr({ workspaceId, refresh }));
  },
  setWorkerConfig(model: string, reasoningEffort: string): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_set_worker_config", withAddr({ model, reasoningEffort }));
  },
  getResearchMap(workspaceId: string): Promise<ResearchMapBundle> {
    return invoke("service_labcontext_get_research_map", withAddr({ workspaceId }));
  },
  initializeResearchMap(workspaceId: string): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_initialize_research_map", withAddr({ workspaceId }));
  },
  saveResearchMapLayout(workspaceId: string, layout: Pick<ResearchMapLayout, "nodes" | "viewport">): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_save_research_map_layout", withAddr({ workspaceId, layout }));
  },
  applyResearchMapPatch(workspaceId: string, patch: ResearchMapPatch): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_apply_research_map_patch", withAddr({ workspaceId, patch }));
  },
  reviewResearchMap(workspaceId: string, preferQueue = true): Promise<{ status: string; proposalId?: string; session?: { sessionId: string; updatedAt: string; workspaceMatch: string } | null; message: string }> {
    return invoke("service_labcontext_review_research_map", withAddr({ workspaceId, preferQueue }));
  },
  researchMapProposalAction(workspaceId: string, proposalId: string, action: "apply" | "reject"): Promise<Record<string, unknown>> {
    return invoke("service_labcontext_research_map_proposal_action", withAddr({ workspaceId, proposalId, action }));
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
