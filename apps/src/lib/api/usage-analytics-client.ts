import { invoke, withAddr } from "./transport";

export interface AnalyticsTotals {
  credits: number | null;
  turns: number | null;
  uncachedInputTokens: number | null;
  cachedInputTokens: number | null;
  outputTokens: number | null;
  totalTokens: number | null;
}
export interface AnalyticsDay { date: string; totals: AnalyticsTotals; capturedAt: number }
export interface PricingState {
  attemptedAt: number | null;
  error: string | null;
  snapshot: {
    fetchedAt: number; usdPerCredit: number; apiSource: string; creditSource: string;
    apiSha256: string; creditSha256: string;
    evidence: { model: string; apiUsdPerMillion: number[]; creditsPerMillion: number[] }[];
  } | null;
}
export interface AnalyticsReport {
  accountId: string;
  source: string;
  startDate: string;
  endDate: string;
  days: AnalyticsDay[];
  sync: { attemptedAt: number; succeededAt: number | null; error: string | null; startDate: string | null; endDate: string | null } | null;
  usdPerCredit: number | null;
  pricing?: PricingState;
}
export interface AnalyticsRange { accountId: string; startDate: string; endDate: string }
export const usageAnalyticsClient = {
  read: (params: AnalyticsRange) => invoke<AnalyticsReport>("service_usage_analytics_read", withAddr({ ...params })),
  refresh: (params: AnalyticsRange) => invoke<AnalyticsReport>("service_usage_analytics_refresh", withAddr({ ...params }), { timeoutMs: 60_000, retries: 0 }),
  setRate: (usdPerCredit: number | null) => invoke("service_usage_analytics_set_rate", withAddr({ usdPerCredit })),
  refreshPricing: (force = false) => invoke<PricingState>("service_usage_analytics_pricing_refresh", withAddr({ force }), { timeoutMs: 60_000, retries: 0 }),
};
