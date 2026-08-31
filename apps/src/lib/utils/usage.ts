"use client";

import type { AccountUsage, AvailabilityLevel } from "@/types";

export type UsageWindowDisplayMode = "primary-only" | "secondary-only" | "dual" | "unknown";

export function toNullableNumber(value: unknown): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function remainingPercent(used: unknown): number | null {
  const value = toNullableNumber(used);
  return value == null ? null : Math.max(0, Math.min(100, Math.round(100 - value)));
}

export function getUsageWindowDisplayMode(usage?: Partial<AccountUsage> | null): UsageWindowDisplayMode {
  const primary = toNullableNumber(usage?.usedPercent) != null || toNullableNumber(usage?.windowMinutes) != null;
  const secondary = toNullableNumber(usage?.secondaryUsedPercent) != null || toNullableNumber(usage?.secondaryWindowMinutes) != null;
  if (!primary && !secondary) return "unknown";
  if (primary && secondary) return "dual";
  if (primary && (toNullableNumber(usage?.windowMinutes) ?? 0) > 24 * 60 + 3) return "secondary-only";
  return primary ? "primary-only" : "secondary-only";
}

export function getUsageDisplayBuckets(usage?: Partial<AccountUsage> | null): {
  mode: UsageWindowDisplayMode;
  primaryRemainPercent: number | null;
  primaryResetsAt: number | null;
  secondaryRemainPercent: number | null;
  secondaryResetsAt: number | null;
} {
  const mode = getUsageWindowDisplayMode(usage);
  if (mode === "secondary-only") {
    const usesPrimaryFields = toNullableNumber(usage?.usedPercent) != null;
    return {
      mode,
      primaryRemainPercent: null,
      primaryResetsAt: null,
      secondaryRemainPercent: remainingPercent(usesPrimaryFields ? usage?.usedPercent : usage?.secondaryUsedPercent),
      secondaryResetsAt: toNullableNumber(usesPrimaryFields ? usage?.resetsAt : usage?.secondaryResetsAt),
    };
  }
  return {
    mode,
    primaryRemainPercent: remainingPercent(usage?.usedPercent),
    primaryResetsAt: toNullableNumber(usage?.resetsAt),
    secondaryRemainPercent: remainingPercent(usage?.secondaryUsedPercent),
    secondaryResetsAt: toNullableNumber(usage?.secondaryResetsAt),
  };
}

export function calcAvailability(
  usage?: Partial<AccountUsage> | null,
  account?: { status?: string; statusReason?: string; hasToken?: boolean } | null,
): { text: string; level: AvailabilityLevel } {
  const status = String(account?.status || "").trim().toLowerCase();
  const reason = String(account?.statusReason || "").trim().toLowerCase();
  if (status === "disabled") return { text: "已禁用", level: "bad" };
  if (status === "banned" || reason.includes("deactivated")) return { text: "封禁", level: "bad" };
  if (["inactive", "unavailable"].includes(status)) return { text: "不可用", level: "bad" };
  if (status === "limited") return { text: "限流", level: "bad" };
  if (account?.hasToken === false) return { text: "缺少授权 Token", level: "bad" };
  if (!usage) return { text: "未知", level: "unknown" };
  const reported = String(usage.availabilityStatus || "").trim().toLowerCase();
  const buckets = getUsageDisplayBuckets(usage);
  const exhausted = [buckets.primaryRemainPercent, buckets.secondaryRemainPercent].some((value) => value === 0);
  if (reported === "unavailable" || exhausted) return { text: "限流", level: "bad" };
  if (reported === "unknown") return { text: "未知", level: "unknown" };
  return { text: "可用", level: "ok" };
}

export function isLowQuotaUsage(usage?: Partial<AccountUsage> | null): boolean {
  const buckets = getUsageDisplayBuckets(usage);
  return [buckets.primaryRemainPercent, buckets.secondaryRemainPercent]
    .some((value) => value != null && value > 0 && value <= 20);
}
