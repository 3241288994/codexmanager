"use client";

import { useQuery } from "@tanstack/react-query";
import { probeLabContextRouter } from "@/lib/api/labcontext-router-client";

export function useLabContextRouter(enabled: boolean) {
  const query = useQuery({
    queryKey: ["labcontext-router", "providers"],
    queryFn: probeLabContextRouter,
    enabled,
    retry: false,
    staleTime: 5_000,
    refetchInterval: 10_000,
    refetchIntervalInBackground: false,
  });

  return {
    ...query,
    localAvailable: query.data?.localAvailable === true,
  };
}
