import type { WebCommandDescriptor } from "./shared";

export function createLabContextWebCommands(): Record<string, WebCommandDescriptor> {
  return {
    service_labcontext_overview: { rpcMethod: "labcontext/overview" },
    service_labcontext_set_default_workspace: { rpcMethod: "labcontext/setDefaultWorkspace" },
    service_labcontext_refresh_workspace: { rpcMethod: "labcontext/refreshWorkspace" },
    service_labcontext_test_tool: { rpcMethod: "labcontext/testTool" },
    service_labcontext_set_tool_policy: { rpcMethod: "labcontext/setToolPolicy" },
    service_labcontext_upsert_workspace: { rpcMethod: "labcontext/upsertWorkspace" },
    service_labcontext_delete_workspace: { rpcMethod: "labcontext/deleteWorkspace" },
    service_labcontext_set_workspace_overview: { rpcMethod: "labcontext/setWorkspaceOverview" },
    service_labcontext_generate_workspace_overview: { rpcMethod: "labcontext/generateWorkspaceOverview" },
    service_labcontext_set_worker_config: { rpcMethod: "labcontext/setWorkerConfig" },
    service_labcontext_get_research_map: { rpcMethod: "labcontext/getResearchMap" },
    service_labcontext_initialize_research_map: { rpcMethod: "labcontext/initializeResearchMap" },
    service_labcontext_save_research_map_layout: { rpcMethod: "labcontext/saveResearchMapLayout" },
    service_labcontext_apply_research_map_patch: { rpcMethod: "labcontext/applyResearchMapPatch" },
    service_labcontext_review_research_map: { rpcMethod: "labcontext/reviewResearchMap" },
    service_labcontext_research_map_proposal_action: { rpcMethod: "labcontext/researchMapProposalAction" },
  };
}
