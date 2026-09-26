"use client";
import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { accountClient } from "@/lib/api/account-client";
import { codexProfileClient } from "@/lib/api/codex-profile-client";
import { usageAnalyticsClient } from "@/lib/api/usage-analytics-client";
import { offsetDate } from "@/lib/usage-analytics";

export function useUsageAnalytics() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, []);
  const today = new Date(now).toISOString().slice(0, 10);
  const [selectedAccount, setSelectedAccount] = useState("");
  const [startDate, setStartDate] = useState(() => offsetDate(today, -29));
  const [lastDate, setLastDate] = useState(today);
  const accounts = useQuery({ queryKey: ["personal", "accounts"], queryFn: accountClient.list });
  const profile = useQuery({ queryKey: ["personal", "profile"], queryFn: () => codexProfileClient.get(), staleTime: 15_000 });
  const items = accounts.data?.items || [];
  const activeId = profile.data?.identityConsistent ? profile.data.selectedAccountId : null;
  const accountId = items.some(a => a.id === selectedAccount) ? selectedAccount : profile.isPending ? "" :
    items.find(a => a.id === activeId)?.id || items.find(a => a.status === "active" && a.hasToken)?.id || items[0]?.id || "";
  const endDate = lastDate ? offsetDate(lastDate, 1) : "";
  const rangeDays = (Date.parse(endDate) - Date.parse(startDate)) / 86400000;
  const validRange = Boolean(startDate && lastDate && rangeDays > 0 && rangeDays <= 366 && lastDate <= today);
  const params = { accountId, startDate, endDate };
  const key = ["usage-analytics", accountId, startDate, endDate];
  const client = useQueryClient();
  const pricing = useQuery({
    queryKey: ["usage-analytics-pricing"], queryFn: () => usageAnalyticsClient.refreshPricing(),
    staleTime: 3_600_000, refetchInterval: 3_600_000, retry: false,
  });
  const refreshPricing = useMutation({
    mutationFn: () => usageAnalyticsClient.refreshPricing(true),
    onSuccess: data => client.setQueryData(["usage-analytics-pricing"], data),
  });
  const report = useQuery({ queryKey: key, queryFn: () => usageAnalyticsClient.read(params), enabled: Boolean(accountId) && validRange });
  const refresh = useMutation({
    mutationFn: usageAnalyticsClient.refresh,
    onSuccess: (data, p) => client.setQueryData(["usage-analytics", p.accountId, p.startDate, p.endDate], data),
    onSettled: (_data, _error, p) => client.invalidateQueries({ queryKey: ["usage-analytics", p.accountId] }),
  });
  const attempted = useRef(new Map<string, number>());
  useEffect(() => {
    if (!accountId || !validRange || !report.data) return;
    const run = () => {
      if (document.visibilityState !== "visible" || refresh.isPending) return;
      const identity = JSON.stringify(params);
      const sync = report.data?.sync;
      const covers = sync?.startDate && sync.endDate && sync.startDate <= startDate && sync.endDate >= endDate;
      const previous = Math.max(attempted.current.get(identity) || 0, covers ? (sync?.attemptedAt || 0) * 1000 : 0);
      if (Date.now() - previous < 600_000) return;
      attempted.current.set(identity, Date.now());
      refresh.mutate(params);
    };
    run();
    const timer = window.setInterval(run, 60_000);
    return () => window.clearInterval(timer);
  }, [accountId, startDate, endDate, validRange, report.data, refresh.isPending]); // eslint-disable-line react-hooks/exhaustive-deps
  return { accounts, accountId, setSelectedAccount, today, now, startDate, setStartDate, lastDate, setLastDate, validRange, params, report, refresh, pricing, refreshPricing };
}
