"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { labContextClient } from "@/lib/api/labcontext-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import type {
  LabContextLocation,
  LabContextWorkspace,
} from "@/types/labcontext";

export const LABCONTEXT_LOCATION_LABELS: Record<LabContextLocation, string> = {
  server: "服务器",
  local: "本地电脑",
};

export type WorkspaceForm = {
  name: string;
  root: string;
  location: LabContextLocation;
};

export type WorkspaceOverviewEditor = {
  location: LabContextLocation;
  workspaceId: string;
  name: string;
  overview: string;
};

export type LabContextTestResult = {
  tool: string;
  input: unknown;
  result: unknown;
  responseBytes: number;
  elapsedMs: number;
  testedAt: string;
};

const EMPTY_WORKSPACE_FORM: WorkspaceForm = {
  name: "",
  root: "",
  location: "server",
};

function suggestedWorkspaceName(root: string): string {
  return root.split(/[\\/]/).filter(Boolean).at(-1) || "";
}

export function useLabContextWorkspace(isDesktopRuntime: boolean) {
  const queryClient = useQueryClient();
  const [location, setLocation] = useState<LabContextLocation>("server");
  const [selectedId, setSelectedId] = useState<string | null>(() => (
    typeof window === "undefined"
      ? null
      : window.localStorage.getItem("labcontext-selected-workspace-server")
  ));
  const [workspaceDialog, setWorkspaceDialog] = useState(false);
  const [workspaceForm, setWorkspaceForm] = useState<WorkspaceForm>(EMPTY_WORKSPACE_FORM);
  const [overviewEditor, setOverviewEditor] = useState<WorkspaceOverviewEditor | null>(null);
  const [testResult, setTestResult] = useState<LabContextTestResult | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    workspace: LabContextWorkspace;
    x: number;
    y: number;
  } | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<LabContextWorkspace | null>(null);

  const overviewQuery = useQuery({
    queryKey: ["labcontext", location, "overview"],
    queryFn: () => labContextClient.overview(location),
    enabled: location === "server" || isDesktopRuntime,
    // Keep the control plane fresh while visible without polling in a hidden tab.
    refetchInterval: () => (
      typeof document !== "undefined" && document.visibilityState === "visible"
        ? 15_000
        : false
    ),
    refetchIntervalInBackground: false,
    staleTime: 5_000,
    retry: location === "server" ? 2 : false,
  });

  useEffect(() => {
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("blur", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("blur", close);
      window.removeEventListener("scroll", close, true);
    };
  }, []);

  const refresh = (targetLocation: LabContextLocation = location) => (
    queryClient.invalidateQueries({ queryKey: ["labcontext", targetLocation] })
  );

  const switchLocation = (next: LabContextLocation) => {
    if (next === "local" && !isDesktopRuntime) return;
    setLocation(next);
    setSelectedId(window.localStorage.getItem(`labcontext-selected-workspace-${next}`));
    setContextMenu(null);
    setOverviewEditor(null);
    setTestResult(null);
    setDeleteTarget(null);
    setWorkspaceDialog(false);
  };

  const defaultMutation = useMutation({
    mutationFn: labContextClient.setDefaultWorkspace,
    onSuccess: async (_, target) => {
      await refresh(target.location);
      toast.success(`${LABCONTEXT_LOCATION_LABELS[target.location]}连接的默认工作区已更新；之后省略 workspace_id 的新调用将使用它`);
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const refreshMutation = useMutation({
    mutationFn: labContextClient.refreshWorkspace,
    onSuccess: async (_, target) => {
      await refresh(target.location);
      toast.success("工作区索引与覆盖信息已刷新");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const policyMutation = useMutation({
    mutationFn: ({
      location: targetLocation,
      profile,
      disabledTools,
    }: {
      location: LabContextLocation;
      profile: string;
      disabledTools: string[];
    }) => labContextClient.setToolPolicy(targetLocation, profile, disabledTools),
    onSuccess: async (_, variables) => {
      await refresh(variables.location);
      toast.success("工具策略已生效");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const testMutation = useMutation({
    mutationFn: ({
      tool,
      target,
    }: {
      tool: string;
      target?: LabContextWorkspace;
    }) => labContextClient.testTool(tool, target),
    onSuccess: (result) => setTestResult(result),
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const workspaceMutation = useMutation({
    mutationFn: labContextClient.upsertWorkspace,
    onSuccess: async (result, variables) => {
      setWorkspaceDialog(false);
      setWorkspaceForm({ ...EMPTY_WORKSPACE_FORM, location: variables.location });
      setLocation(variables.location);
      setSelectedId(result.workspaceId);
      await refresh(variables.location);
      toast.success(`${LABCONTEXT_LOCATION_LABELS[variables.location]}工作区已添加，Codex 正在生成首版概述`);
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const overviewMutation = useMutation({
    mutationFn: ({
      location: targetLocation,
      workspaceId,
      overview,
    }: WorkspaceOverviewEditor) => labContextClient.setWorkspaceOverview(
      { location: targetLocation, workspaceId },
      overview,
    ),
    onSuccess: async (_, variables) => {
      setOverviewEditor(null);
      await refresh(variables.location);
      toast.success("工作区概述已保存，之后的模型概述会使用它");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const generateOverviewMutation = useMutation({
    mutationFn: ({
      target,
      refresh: force,
    }: {
      target: LabContextWorkspace;
      refresh: boolean;
    }) => labContextClient.generateWorkspaceOverview(target, force),
    onSuccess: async (result, variables) => {
      setOverviewEditor(null);
      await refresh(variables.target.location);
      if (result.status === "completed" || result.status === "ready") {
        toast.success("Codex 概述已生成并写入 context.yaml");
      } else if (result.status === "failed") {
        toast.error(result.error || "Codex 概述生成失败");
      } else {
        toast.success("Codex 概述任务已启动；卡片会自动显示进度和结果");
      }
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const deleteMutation = useMutation({
    mutationFn: labContextClient.deleteWorkspace,
    onSuccess: async (result, target) => {
      setDeleteTarget(null);
      if (selectedId === result.workspaceId) setSelectedId(null);
      await refresh(target.location);
      toast.success("已移除工作区注册；项目目录和文件均未删除");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const workerMutation = useMutation({
    mutationFn: ({
      location: targetLocation,
      model,
      reasoningEffort,
    }: {
      location: LabContextLocation;
      model: string;
      reasoningEffort: string;
    }) => labContextClient.setWorkerConfig(targetLocation, model, reasoningEffort),
    onSuccess: async (_, variables) => {
      await refresh(variables.location);
      toast.success("Codex worker 配置已更新，将用于之后新建的分析任务");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const pickDirectoryMutation = useMutation({
    mutationFn: labContextClient.pickLocalWorkspaceDirectory,
    onSuccess: (result) => {
      if (result.canceled || !result.path) return;
      setWorkspaceForm((value) => ({
        ...value,
        name: value.name.trim() || suggestedWorkspaceName(result.path || ""),
        root: result.path || "",
      }));
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const data = overviewQuery.data;
  const selected = useMemo(
    () => data?.workspaces.find((item) => item.workspaceId === selectedId)
      || data?.workspaces.find((item) => item.workspaceId === data.defaultWorkspaceId)
      || data?.workspaces[0]
      || null,
    [data, selectedId],
  );

  const selectedWorkspaceId = selected?.workspaceId;
  useEffect(() => {
    if (selectedWorkspaceId) {
      window.localStorage.setItem(
        `labcontext-selected-workspace-${location}`,
        selectedWorkspaceId,
      );
    }
  }, [location, selectedWorkspaceId]);

  const setToolEnabled = (name: string, enabled: boolean) => {
    if (!data) return;
    const disabledTools = data.toolPolicy.tools
      .filter((tool) => tool.name !== name ? !tool.enabled : !enabled)
      .map((tool) => tool.name);
    policyMutation.mutate({ location, profile: "custom", disabledTools });
  };

  const openResearchMap = (workspaceId: string) => {
    setSelectedId(workspaceId);
    window.setTimeout(() => {
      document.getElementById("research-map")?.scrollIntoView({
        behavior: "smooth",
        block: "start",
      });
    }, 80);
  };

  const openWorkspaceDialog = () => {
    setWorkspaceForm({ ...EMPTY_WORKSPACE_FORM, location });
    setWorkspaceDialog(true);
  };

  const setWorkspaceFormLocation = (next: LabContextLocation) => {
    if (next === "local" && !isDesktopRuntime) return;
    setWorkspaceForm((value) => ({ ...value, location: next, root: "" }));
  };

  const openContextMenu = (
    workspace: LabContextWorkspace,
    point: { x: number; y: number },
  ) => {
    const width = 280;
    const height = 270;
    setContextMenu({
      workspace,
      x: Math.max(8, Math.min(point.x, window.innerWidth - width - 8)),
      y: Math.max(8, Math.min(point.y, window.innerHeight - height - 8)),
    });
  };

  return {
    location,
    switchLocation,
    overviewQuery,
    data,
    selected,
    selectedId,
    setSelectedId,
    workspaceDialog,
    setWorkspaceDialog,
    workspaceForm,
    setWorkspaceForm,
    setWorkspaceFormLocation,
    openWorkspaceDialog,
    overviewEditor,
    setOverviewEditor,
    testResult,
    setTestResult,
    contextMenu,
    setContextMenu,
    openContextMenu,
    deleteTarget,
    setDeleteTarget,
    setToolEnabled,
    openResearchMap,
    defaultMutation,
    refreshMutation,
    policyMutation,
    testMutation,
    workspaceMutation,
    overviewMutation,
    generateOverviewMutation,
    deleteMutation,
    workerMutation,
    pickDirectoryMutation,
  };
}
