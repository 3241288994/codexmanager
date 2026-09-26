import type { AnalyticsDay, AnalyticsReport, AnalyticsTotals } from "./api/usage-analytics-client";

export function valuation(report: AnalyticsReport | undefined) {
  const manual = report?.usdPerCredit;
  const snapshot = report?.pricing?.snapshot;
  return {
    rate: manual ?? snapshot?.usdPerCredit ?? null,
    method: manual != null ? "manual-credit-rate" : snapshot ? "official-tables-derived-reference" : "unavailable",
  };
}

export function referenceAmount(credits: number | null | undefined, rate: number | null): number | null {
  return credits == null || rate == null ? null : credits * rate;
}

export function sumMetrics(days: AnalyticsDay[]): AnalyticsTotals {
  const sum = (key: keyof AnalyticsTotals) => days.length && days.every(d => d.totals[key] != null)
    ? days.reduce((n, d) => n + (d.totals[key] ?? 0), 0) : null;
  return { credits: sum("credits"), turns: sum("turns"), uncachedInputTokens: sum("uncachedInputTokens"), cachedInputTokens: sum("cachedInputTokens"), outputTokens: sum("outputTokens"), totalTokens: sum("totalTokens") };
}

export function cacheRatio(t: AnalyticsTotals): number | null {
  if (t.cachedInputTokens == null || t.uncachedInputTokens == null) return null;
  const input = t.cachedInputTokens + t.uncachedInputTokens;
  return input > 0 ? t.cachedInputTokens / input : null;
}

export function offsetDate(date: string, days: number): string {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

export function analyticsCsv(report: AnalyticsReport): string {
  const escape = (value: unknown) => `"${String(value ?? "").replaceAll('"', '""')}"`;
  const v = valuation(report);
  const snapshot = report.pricing?.snapshot;
  const rows: unknown[][] = [["date", "credits", "uncached_input_tokens", "cached_input_tokens", "output_tokens", "total_tokens", "turns", "cache_ratio", "estimated_usd", "captured_at", "source", "valuation_method", "usd_per_credit", "pricing_fetched_at", "api_price_source", "credit_price_source", "not_actual_bill"]];
  for (const day of report.days) {
    const t = day.totals;
    rows.push([day.date, t.credits, t.uncachedInputTokens, t.cachedInputTokens, t.outputTokens, t.totalTokens, t.turns, cacheRatio(t), referenceAmount(t.credits, v.rate), new Date(day.capturedAt * 1000).toISOString(), report.source, v.method, v.rate, snapshot ? new Date(snapshot.fetchedAt * 1000).toISOString() : null, snapshot?.apiSource, snapshot?.creditSource, true]);
  }
  return "\uFEFF" + rows.map(row => row.map(escape).join(",")).join("\r\n");
}

export function downloadReport(report: AnalyticsReport, format: "json" | "csv") {
  const v = valuation(report);
  const exported = { ...report, valuation: { ...v, estimatedUsd: referenceAmount(sumMetrics(report.days).credits, v.rate), notActualBill: true }, days: report.days.map(d => ({ ...d, estimatedUsd: referenceAmount(d.totals.credits, v.rate) })) };
  const blob = new Blob([format === "csv" ? analyticsCsv(report) : JSON.stringify(exported, null, 2)], { type: format === "csv" ? "text/csv;charset=utf-8" : "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `codex-usage-${report.accountId.replace(/[^a-zA-Z0-9_-]/g, "_")}-${report.startDate}-${report.endDate}.${format}`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
