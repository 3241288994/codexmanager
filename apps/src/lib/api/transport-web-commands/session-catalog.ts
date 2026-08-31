import type { WebCommandDescriptor } from "./shared";

export function createSessionCatalogWebCommands(): Record<string, WebCommandDescriptor> {
  return {
    service_session_catalog_list: { rpcMethod: "sessionCatalog/list" },
    service_session_catalog_repair_provider_index: { rpcMethod: "sessionCatalog/repairProviderIndex" },
  };
}
