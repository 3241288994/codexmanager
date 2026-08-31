import type { WebCommandDescriptor } from "./shared";

export function createLoginWebCommands(): Record<string, WebCommandDescriptor> {
  return {
    service_login_start: {
      rpcMethod: "account/login/start",
      mapParams: (params) => ({
        ...(params ?? {}),
        type:
          typeof params?.loginType === "string" && params.loginType.trim()
            ? params.loginType
            : "chatgpt",
        openBrowser: false,
      }),
    },
    service_login_status: { rpcMethod: "account/login/status" },
    service_login_complete: { rpcMethod: "account/login/complete" },
  };
}
