import type { WebCommandDescriptor } from "./shared";

export function createAccountWebCommands(): Record<string, WebCommandDescriptor> {
  return {
    service_usage_analytics_read: { rpcMethod: "account/analytics/read" },
    service_usage_analytics_refresh: { rpcMethod: "account/analytics/refresh" },
    service_usage_analytics_set_rate: { rpcMethod: "account/analytics/setRate" },
    service_usage_analytics_pricing_refresh: { rpcMethod: "account/analytics/pricingRefresh" },
    service_usage_polling_status: { rpcMethod: "account/usage/pollingStatus" },
    service_account_list: { rpcMethod: "account/list" },
    service_usage_list: { rpcMethod: "account/usage/list" },
    service_usage_refresh: { rpcMethod: "account/usage/refresh" },
  };
}
