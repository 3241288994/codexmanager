import { invoke, withAddr } from "./transport";
import type { ServiceInitializationResult } from "../../types";
import { readInitializeResult } from "@/lib/utils/service";

export const serviceClient = {
  start: (addr?: string) => invoke("service_start", { addr }),
  async initialize(addr?: string): Promise<ServiceInitializationResult> {
    const result = await invoke<unknown>("service_initialize", addr ? { addr } : withAddr());
    return readInitializeResult(result);
  },
};
