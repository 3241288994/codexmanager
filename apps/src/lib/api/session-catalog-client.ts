import { invoke, withAddr } from "./transport";

export type SessionVisibility =
  | "ready"
  | "missing_rollout"
  | "provider_mismatch"
  | "missing_workspace";

export interface SessionCatalogItem {
  id: string;
  title: string;
  cwd: string | null;
  rolloutPath: string | null;
  modelProvider: string | null;
  model: string | null;
  reasoningEffort: string | null;
  updatedAtMs: number;
  archived: boolean;
  rolloutExists: boolean;
  cwdExists: boolean;
  visibility: SessionVisibility;
  resumeCommand: string;
}

export interface SessionCatalogResult {
  codexHome: string;
  items: SessionCatalogItem[];
  total: number;
  diagnostics: {
    stateDbAvailable: boolean;
    checkedCount: number;
    providerMismatchCount: number;
    missingRolloutCount: number;
    missingCwdCount: number;
    message: string;
  };
}

export interface SessionIndexRepairResult {
  sessionId: string;
  previousProvider: string | null;
  targetProvider: string;
  updatedRows: number;
  backupPath: string;
  ledgerPath: string;
  message: string;
}

export const sessionCatalogClient = {
  list(params?: {
    codexHome?: string | null;
    query?: string;
    includeArchived?: boolean;
    limit?: number;
  }): Promise<SessionCatalogResult> {
    return invoke<SessionCatalogResult>(
      "service_session_catalog_list",
      withAddr({
        codexHome: params?.codexHome || null,
        query: params?.query || null,
        includeArchived: params?.includeArchived ?? false,
        limit: params?.limit ?? 200,
      }),
    );
  },
  repairProviderIndex(sessionId: string, codexHome?: string | null): Promise<SessionIndexRepairResult> {
    return invoke<SessionIndexRepairResult>(
      "service_session_catalog_repair_provider_index",
      withAddr({
        codexHome: codexHome || null,
        sessionId,
        confirmSessionId: sessionId,
      }),
    );
  },
};
