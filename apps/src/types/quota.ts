export interface QuotaOverviewResult {
  apiKey: {
    keyCount: number;
    limitedKeyCount: number;
    totalLimitTokens: number | null;
    totalUsedTokens: number;
    totalRemainingTokens: number | null;
    estimatedCostUsd: number;
  };
  aggregateApi: {
    sourceCount: number;
    enabledBalanceQueryCount: number;
    okCount: number;
    errorCount: number;
    totalBalanceUsd: number | null;
    lastRefreshedAt: number | null;
  };
  openaiAccount: {
    accountCount: number;
    availableCount: number;
    lowQuotaCount: number;
    primaryRemainPercent: number | null;
    secondaryRemainPercent: number | null;
    lastRefreshedAt: number | null;
  };
  todayUsage: {
    inputTokens: number;
    cachedInputTokens: number;
    outputTokens: number;
    reasoningOutputTokens: number;
    totalTokens: number;
    estimatedCostUsd: number;
  };
}

export interface BillingRule {
  id: string;
  name: string;
  status: "active" | "disabled" | string;
  priority: number;
  multiplierMillis: number;
  modelPattern: string | null;
  serviceTier: string | null;
  userId: string | null;
  projectId: string | null;
  apiKeyId: string | null;
  startsAt: number | null;
  endsAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface BillingRuleUpsertParams {
  id?: string | null;
  name: string;
  status?: "active" | "disabled" | string | null;
  priority?: number | null;
  multiplierMillis: number;
  modelPattern?: string | null;
  serviceTier?: string | null;
  userId?: string | null;
  apiKeyId?: string | null;
  startsAt?: number | null;
  endsAt?: number | null;
}

export interface QuotaModelUsageItem {
  model: string;
  provider: string | null;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number | null;
  priceStatus: string;
  apiKeyRemainingTokens: number | null;
  aggregateEstimatedRemainingTokens: number | null;
  aggregateBalanceUsd: number | null;
  openaiAvailableAccountCount: number;
  openaiPrimaryRemainPercent: number | null;
  openaiSecondaryRemainPercent: number | null;
  openaiEstimatedRemainingTokens: number | null;
  openaiEstimateEnabled: boolean;
}

export interface QuotaApiKeyModelUsageItem {
  model: string;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number | null;
  priceStatus: string;
}

export interface QuotaApiKeyUsageItem {
  keyId: string;
  name: string | null;
  modelSlug: string | null;
  quotaLimitTokens: number | null;
  usedTokens: number;
  remainingTokens: number | null;
  estimatedCostUsd: number;
  models: QuotaApiKeyModelUsageItem[];
}

export interface QuotaSourceSummary {
  id: string;
  kind: "api_key" | "aggregate_api" | "openai_account" | string;
  name: string;
  status: string;
  metricKind: "token_limit" | "money_balance" | "window_percent" | string;
  remaining: number | null;
  total: number | null;
  used: number | null;
  unit: string | null;
  models: string[];
  provider: string | null;
  capturedAt: number | null;
  error: string | null;
}

export interface QuotaRefreshSourcesParams {
  kinds?: Array<"aggregate_api" | "openai_account">;
  sourceIds?: string[];
}

export interface QuotaRefreshSourceResult {
  id: string;
  kind: string;
  ok: boolean;
  error: string | null;
}

export interface QuotaSourceModelAssignment {
  sourceKind: string;
  sourceId: string;
  modelSlugs: string[];
}

export interface AccountQuotaCapacityTemplate {
  planType: string;
  primaryWindowTokens: number | null;
  secondaryWindowTokens: number | null;
}

export interface AccountQuotaCapacityOverride {
  accountId: string;
  primaryWindowTokens: number | null;
  secondaryWindowTokens: number | null;
}

export interface QuotaCapacityConfigResult {
  sourceAssignments: QuotaSourceModelAssignment[];
  templates: AccountQuotaCapacityTemplate[];
  accountOverrides: AccountQuotaCapacityOverride[];
}

export interface QuotaPoolSourceBreakdown {
  sourceKind: string;
  sourceId: string;
  name: string;
  status: string;
  remainingTokens: number | null;
  rawRemaining: number | null;
  rawUnit: string | null;
  models: string[];
  capturedAt: number | null;
  priceStatus: string;
}

export interface QuotaModelPoolItem {
  model: string;
  provider: string | null;
  totalRemainingTokens: number | null;
  aggregateRemainingTokens: number | null;
  accountPrimaryRemainingTokens: number | null;
  accountSecondaryRemainingTokens: number | null;
  accountEstimatedRemainingTokens: number | null;
  sourceCount: number;
  sources: QuotaPoolSourceBreakdown[];
  priceStatus: string;
}

export interface QuotaModelPoolsResult {
  items: QuotaModelPoolItem[];
  templates: AccountQuotaCapacityTemplate[];
  accountOverrides: AccountQuotaCapacityOverride[];
}

export interface QuotaSystemPoolResult {
  referenceModel: string;
  provider: string | null;
  totalRemainingTokens: number | null;
  aggregateRemainingTokens: number | null;
  accountPrimaryRemainingTokens: number | null;
  accountSecondaryRemainingTokens: number | null;
  accountEstimatedRemainingTokens: number | null;
  aggregateSourceCount: number;
  accountSourceCount: number;
  unknownSourceCount: number;
  priceStatus: string;
  sources: QuotaPoolSourceBreakdown[];
}
