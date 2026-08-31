"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Activity, AlertTriangle, Bot, CheckCircle2, FolderGit2,
  BrainCircuit, MoreHorizontal, Pencil, Plus, RefreshCw, Server, Sparkles, Star, Trash2, Wrench,
} from "lucide-react";
import { toast } from "sonner";
import { labContextClient } from "@/lib/api/labcontext-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import type { LabContextHealthState, LabContextWorkspace } from "@/types/labcontext";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ResearchMapPanel } from "@/components/labcontext/research-map-panel";

const HEALTH_LABELS: Record<LabContextHealthState, string> = {
  healthy: "正常", degraded: "部分降级", unknown: "无法观测", down: "异常",
};
const HEALTH_VARIANTS: Record<LabContextHealthState, "default" | "secondary" | "outline" | "destructive"> = {
  healthy: "default", degraded: "secondary", unknown: "outline", down: "destructive",
};
const TOOL_LATENCY = { instant: "即时", indexed: "索引查询", codex: "Codex 分析" } as const;

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  return `${(value / 1024 ** 3).toFixed(1)} GB`;
}

function formatTime(value?: string | null): string {
  return value ? new Date(value).toLocaleString() : "未知";
}

function WorkspaceMenu({ workspace, onMap, onDefault, onRefresh, onGenerate, onDelete }: {
  workspace: LabContextWorkspace;
  onMap: () => void;
  onDefault: () => void;
  onRefresh: () => void;
  onGenerate: () => void;
  onDelete: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger><Button variant="ghost" size="icon-sm" aria-label={`${workspace.name} 操作`}><MoreHorizontal /></Button></DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-64">
        <DropdownMenuItem className="gap-2 whitespace-nowrap" onClick={onMap}><BrainCircuit className="size-4" />打开研究图</DropdownMenuItem>
        <DropdownMenuItem className="gap-2 whitespace-nowrap" onClick={onDefault} disabled={workspace.isDefault}><Star className="size-4" />设为 ChatGPT 默认</DropdownMenuItem>
        <DropdownMenuItem className="gap-2 whitespace-nowrap" onClick={onRefresh}><RefreshCw className="size-4" />刷新索引与覆盖</DropdownMenuItem>
        <DropdownMenuItem className="gap-2 whitespace-nowrap" onClick={onGenerate}><Sparkles className="size-4" />用 Codex 重新生成概述</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={onDelete} disabled={workspace.isDefault} className="gap-2 whitespace-nowrap text-destructive focus:text-destructive"><Trash2 className="size-4" />删除工作区注册</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default function LabContextPage() {
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(() => (
    typeof window === "undefined" ? null : window.localStorage.getItem("labcontext-selected-workspace")
  ));
  const [workspaceDialog, setWorkspaceDialog] = useState(false);
  const [overviewEditor, setOverviewEditor] = useState<{ workspaceId: string; name: string; overview: string } | null>(null);
  const [testResult, setTestResult] = useState<{ tool: string; input: unknown; result: unknown; responseBytes: number; elapsedMs: number; testedAt: string } | null>(null);
  const [workspaceForm, setWorkspaceForm] = useState({ name: "", root: "" });
  const [contextMenu, setContextMenu] = useState<{ workspace: LabContextWorkspace; x: number; y: number } | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<LabContextWorkspace | null>(null);
  const overviewQuery = useQuery({
    queryKey: ["labcontext", "overview"],
    queryFn: () => labContextClient.overview(),
    refetchInterval: 15_000,
  });

  useEffect(() => {
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("blur", close);
    return () => { window.removeEventListener("click", close); window.removeEventListener("blur", close); };
  }, []);

  const refresh = async () => queryClient.invalidateQueries({ queryKey: ["labcontext"] });
  const defaultMutation = useMutation({
    mutationFn: labContextClient.setDefaultWorkspace,
    onSuccess: async () => { await refresh(); toast.success("默认工作区已更新；之后省略 workspace_id 的新调用将使用它，已开始的 ChatGPT 调用不会追溯变更"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const refreshMutation = useMutation({
    mutationFn: labContextClient.refreshWorkspace,
    onSuccess: async () => { await refresh(); toast.success("工作区索引与覆盖信息已刷新"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const policyMutation = useMutation({
    mutationFn: ({ profile, disabledTools }: { profile: string; disabledTools: string[] }) => labContextClient.setToolPolicy(profile, disabledTools),
    onSuccess: async () => { await refresh(); toast.success("工具策略已生效"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const testMutation = useMutation({
    mutationFn: ({ tool, workspaceId }: { tool: string; workspaceId?: string }) => labContextClient.testTool(tool, workspaceId),
    onSuccess: (result) => setTestResult(result),
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const workspaceMutation = useMutation({
    mutationFn: () => labContextClient.upsertWorkspace(workspaceForm),
    onSuccess: async (result) => {
      setWorkspaceDialog(false);
      setWorkspaceForm({ name: "", root: "" });
      setSelectedId(result.workspaceId);
      await refresh();
      toast.success("工作区已添加，Codex 正在结合项目文件和最近会话生成首版概述");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const overviewMutation = useMutation({
    mutationFn: ({ workspaceId, overview }: { workspaceId: string; overview: string }) => labContextClient.setWorkspaceOverview(workspaceId, overview),
    onSuccess: async () => { setOverviewEditor(null); await refresh(); toast.success("工作区概述已保存，之后的模型概述会使用它"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const generateOverviewMutation = useMutation({
    mutationFn: ({ workspaceId, refresh: force }: { workspaceId: string; refresh: boolean }) => labContextClient.generateWorkspaceOverview(workspaceId, force),
    onSuccess: async (result) => {
      setOverviewEditor(null);
      await refresh();
      if (result.status === "completed" || result.status === "ready") toast.success("Codex 概述已生成并写入 context.yaml");
      else if (result.status === "failed") toast.error(result.error || "Codex 概述生成失败");
      else toast.success("Codex 概述任务已启动；卡片会自动显示进度和结果");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const deleteMutation = useMutation({
    mutationFn: labContextClient.deleteWorkspace,
    onSuccess: async (result) => {
      setDeleteTarget(null);
      if (selectedId === result.workspaceId) setSelectedId(null);
      await refresh();
      toast.success("已移除工作区注册；项目目录和文件均未删除");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const workerMutation = useMutation({
    mutationFn: ({ model, reasoningEffort }: { model: string; reasoningEffort: string }) => labContextClient.setWorkerConfig(model, reasoningEffort),
    onSuccess: async () => { await refresh(); toast.success("Codex worker 配置已更新，将用于之后新建的分析任务"); },
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
    if (selectedWorkspaceId) window.localStorage.setItem("labcontext-selected-workspace", selectedWorkspaceId);
  }, [selectedWorkspaceId]);
  const setToolEnabled = (name: string, enabled: boolean) => {
    if (!data) return;
    const disabledTools = data.toolPolicy.tools.filter((tool) => tool.name !== name ? !tool.enabled : !enabled).map((tool) => tool.name);
    policyMutation.mutate({ profile: "custom", disabledTools });
  };
  const openResearchMap = (workspaceId: string) => {
    setSelectedId(workspaceId);
    window.setTimeout(() => document.getElementById("research-map")?.scrollIntoView({ behavior: "smooth", block: "start" }), 80);
  };

  if (overviewQuery.isError) {
    return (
      <main className="mx-auto flex w-full max-w-7xl flex-col gap-5 px-4 py-6 md:px-8">
        <header><p className="text-sm font-medium text-primary">CodexManager</p><h1 className="mt-1 text-3xl font-semibold">科研工作区</h1></header>
        <Card className="border-destructive/30"><CardContent className="flex items-start gap-3 p-5"><AlertTriangle className="mt-0.5 size-5 text-destructive" /><div><p className="font-medium">无法连接 LabContext 管理接口</p><p className="mt-1 text-sm text-muted-foreground">{getAppErrorMessage(overviewQuery.error)}</p><Button className="mt-4" variant="outline" onClick={() => overviewQuery.refetch()}><RefreshCw />重试</Button></div></CardContent></Card>
      </main>
    );
  }

  return (
    <main className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-4 py-6 md:px-8">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div><p className="text-sm font-medium text-primary">CodexManager</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">科研工作区</h1><p className="mt-2 text-sm text-muted-foreground">管理 ChatGPT 可查询的科研事实库、工具策略和分析任务。</p></div>
        <div className="flex gap-2"><Button variant="outline" onClick={() => overviewQuery.refetch()} disabled={overviewQuery.isFetching}><RefreshCw className={overviewQuery.isFetching ? "animate-spin" : ""} />刷新状态</Button><Button onClick={() => setWorkspaceDialog(true)}><Plus />添加工作区</Button></div>
      </header>

      {data ? (
        <>
          <Card className="border-primary/20">
            <CardHeader className="pb-3"><div className="flex flex-wrap items-center justify-between gap-2"><div><CardTitle className="flex items-center gap-2 text-base"><Server className="size-4" />服务链状态</CardTitle><CardDescription>只展示能够验证的环节；Mac tunnel 无 heartbeat 时不会伪装成健康。</CardDescription></div><Badge variant={HEALTH_VARIANTS[data.health.overall]}>{HEALTH_LABELS[data.health.overall]}</Badge></div></CardHeader>
            <CardContent className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
              {data.health.checks.map((check) => <div key={check.id} className="rounded-lg border bg-background/50 p-3"><div className="flex items-center justify-between gap-2"><p className="text-sm font-medium">{check.label}</p><span className={`size-2 rounded-full ${check.status === "healthy" ? "bg-emerald-500" : check.status === "down" ? "bg-red-500" : "bg-amber-500"}`} /></div><p className="mt-2 line-clamp-2 text-xs text-muted-foreground" title={check.detail}>{check.detail}</p></div>)}
            </CardContent>
          </Card>

          <section className="space-y-3">
            <div className="flex items-center justify-between"><div><h2 className="text-lg font-semibold">科研工作区</h2><p className="text-xs text-muted-foreground">每张卡片是一项科研项目；悬停查看详情，右键可设为 ChatGPT 默认工作区。</p></div><span className="text-sm text-muted-foreground">{data.workspaces.length} 个</span></div>
            <div className="grid gap-3 lg:grid-cols-2 xl:grid-cols-3">
              {data.workspaces.map((workspace, workspaceIndex) => (
                <Card
                  key={workspace.workspaceId}
                  className={`relative min-w-0 cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-md ${selected?.workspaceId === workspace.workspaceId ? "border-primary/55 bg-primary/[0.035]" : "border-border/70 hover:border-primary/30"}`}
                  onClick={() => setSelectedId(workspace.workspaceId)}
                  onContextMenu={(event) => { event.preventDefault(); setContextMenu({ workspace, x: event.clientX, y: event.clientY }); }}
                >
                  <CardContent className="grid min-h-64 min-w-0 grid-cols-[minmax(0,1fr)] content-start gap-4 overflow-hidden p-5">
                    <div className="peer/workspace-header flex items-start justify-between gap-3"><div className="flex min-w-0 gap-3"><div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted"><FolderGit2 className="size-5" /></div><div className="min-w-0"><div className="flex items-center gap-2"><p className="truncate font-semibold">{workspace.name}</p>{workspace.isDefault ? <Star className="size-4 fill-amber-400 text-amber-500" /> : null}</div><p className="mt-1 truncate font-mono text-xs text-muted-foreground">{workspace.workspaceId} · {workspace.root}</p></div></div><WorkspaceMenu workspace={workspace} onMap={() => openResearchMap(workspace.workspaceId)} onDefault={() => defaultMutation.mutate(workspace.workspaceId)} onRefresh={() => refreshMutation.mutate(workspace.workspaceId)} onGenerate={() => generateOverviewMutation.mutate({ workspaceId: workspace.workspaceId, refresh: true })} onDelete={() => setDeleteTarget(workspace)} /></div>
                    <div className="min-w-0 overflow-hidden">
                      <Tooltip>
                        <TooltipTrigger
                          className="line-clamp-4 w-full min-w-0 max-w-full cursor-help break-words text-left text-sm leading-6 text-muted-foreground [overflow-wrap:anywhere]"
                          aria-label={`${workspace.name} 完整概述`}
                        >
                          {workspace.description}
                        </TooltipTrigger>
                        <TooltipContent
                          side="top"
                          align={workspaceIndex === 0 ? "start" : "end"}
                          sideOffset={8}
                          collisionPadding={16}
                          className="block max-h-[min(24rem,70vh)] w-[min(36rem,calc(100vw-2rem))] max-w-none overflow-y-auto whitespace-normal p-4 text-left text-sm leading-6 [overflow-wrap:anywhere]"
                        >
                          <p className="mb-1 font-medium text-foreground">{workspace.name} · 完整概述</p>
                          <p className="text-muted-foreground">{workspace.description}</p>
                        </TooltipContent>
                      </Tooltip>
                      <div className="mt-1 flex min-w-0 flex-wrap items-center gap-2 text-[11px] text-muted-foreground"><span className="min-w-0 break-words [overflow-wrap:anywhere]">{workspace.overviewSource === "reviewed" ? "人工维护概述" : workspace.overviewSource === "codex" ? "Codex 基于项目与最近会话生成" : workspace.context.generation?.status === "running" || workspace.context.generation?.status === "not_started" ? "等待 Codex 生成首版概述" : "根据项目文件自动提取"}</span>{workspace.context.generation?.status === "running" ? <Badge variant="outline" className="shrink-0 gap-1"><RefreshCw className="size-3 animate-spin" />生成中</Badge> : workspace.context.generation?.status === "failed" ? <Badge variant="destructive" className="shrink-0" title={workspace.context.generation.error || "Codex 概述生成失败，请重试"}>生成失败</Badge> : null}</div>
                    </div>
                    <div className="flex flex-wrap gap-1.5">{workspace.readableAssets.filter((asset) => asset.fileCount > 0).map((asset) => <Badge key={asset.kind} variant="secondary">{asset.label} {asset.fileCount}</Badge>)}</div>
                    <button className="min-w-0 rounded-lg border bg-muted/25 px-3 py-2 text-left transition-colors hover:bg-muted/50" onClick={(event) => { event.stopPropagation(); openResearchMap(workspace.workspaceId); }}><div className="flex items-center justify-between gap-2"><span className="flex items-center gap-1.5 text-xs font-medium"><BrainCircuit className="size-3.5 text-primary" />研究图</span><span className="text-[10px] text-muted-foreground">{workspace.researchMap.status === "ready" ? `${workspace.researchMap.counts.nodes} 节点` : "待初始化"}</span></div><p className="mt-1 truncate text-[11px] text-muted-foreground">{workspace.researchMap.currentTarget?.title || workspace.researchMap.coreIdea?.title || "建立项目的目标、分支与证据关系"}</p>{workspace.researchMap.pendingProposals ? <Badge variant="secondary" className="mt-1.5">{workspace.researchMap.pendingProposals} 个待审提案</Badge> : null}</button>
                    <div className="mt-auto flex min-w-0 items-center justify-between gap-2 border-t pt-3"><p className="min-w-0 truncate text-xs text-muted-foreground">{workspace.git.dirty ? `${workspace.git.changedPathCount} 项未提交变更` : "Git 工作树干净"}</p><Button className="shrink-0" size="sm" variant="ghost" onClick={(event) => { event.stopPropagation(); setOverviewEditor({ workspaceId: workspace.workspaceId, name: workspace.name, overview: workspace.description }); }}><Pencil />编辑概述</Button></div>
                  </CardContent>
                  <div className="pointer-events-none absolute inset-x-3 top-[calc(100%-10px)] z-30 hidden rounded-lg border bg-popover p-4 text-popover-foreground shadow-xl peer-focus-within/workspace-header:block peer-hover/workspace-header:block">
                    <p className="font-medium">模型可以了解什么</p>
                    <div className="mt-2 grid gap-2">{workspace.readableAssets.map((asset) => <div key={asset.kind} className="flex items-start justify-between gap-4 text-xs"><div><p className="font-medium">{asset.label}</p><p className="text-muted-foreground">{asset.meaning}</p></div><span className="shrink-0">{asset.fileCount ? `${asset.fileCount} 项` : "未发现"}</span></div>)}</div>
                    <div className="mt-3 min-w-0 border-t pt-3 text-xs text-muted-foreground"><p className="break-all">路径：{workspace.root}</p><p className="mt-1 break-all">Git：{workspace.git.branch || "不可用"} · {workspace.git.commit || workspace.git.headState}</p><p className="mt-1 break-all">概述文件：{workspace.context.path}</p></div>
                  </div>
                </Card>
              ))}
            </div>
          </section>

          {selected ? (
            <>
            <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
              <Card><CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><div><CardTitle className="text-base">当前查看：{selected.name}</CardTitle><CardDescription>测试可确认 ChatGPT 实际能读到的项目背景；重新扫描会发现新增实验摘要。</CardDescription></div><div className="flex gap-2"><Button variant="outline" size="sm" onClick={() => testMutation.mutate({ tool: "workspace_overview", workspaceId: selected.workspaceId })} disabled={testMutation.isPending}><CheckCircle2 />测试 ChatGPT 所见内容</Button><Button variant="outline" size="sm" onClick={() => refreshMutation.mutate(selected.workspaceId)} disabled={refreshMutation.isPending}><RefreshCw className={refreshMutation.isPending ? "animate-spin" : ""} />重新扫描项目</Button></div></div></CardHeader></Card>
              <Card><CardHeader className="pb-3"><CardTitle className="flex items-center gap-2 text-base"><Bot className="size-4" />Codex analysis worker</CardTitle><CardDescription>仅影响之后新建的深度分析任务；已运行任务不会中途换模型。</CardDescription></CardHeader><CardContent className="grid grid-cols-2 gap-3"><div className="grid gap-1.5"><Label htmlFor="worker-model">模型</Label><select id="worker-model" className="h-9 rounded-lg border bg-background px-3 text-sm" value={data.workerConfig.model} disabled={workerMutation.isPending} onChange={(event) => { const model = event.target.value; const efforts = data.workerConfig.availableEfforts[model] || ["medium"]; const effort = efforts.includes(data.workerConfig.reasoningEffort) ? data.workerConfig.reasoningEffort : "medium"; workerMutation.mutate({ model, reasoningEffort: effort }); }}>{data.workerConfig.availableModels.map((model) => <option key={model} value={model}>{model}</option>)}</select></div><div className="grid gap-1.5"><Label htmlFor="worker-effort">思考强度</Label><select id="worker-effort" className="h-9 rounded-lg border bg-background px-3 text-sm" value={data.workerConfig.reasoningEffort} disabled={workerMutation.isPending} onChange={(event) => workerMutation.mutate({ model: data.workerConfig.model, reasoningEffort: event.target.value })}>{(data.workerConfig.availableEfforts[data.workerConfig.model] || []).map((effort) => <option key={effort} value={effort}>{effort}</option>)}</select></div></CardContent></Card>
            </section>
            <ResearchMapPanel workspace={selected} />
            </>
          ) : null}

          <section className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
            <Card>
              <CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><div><CardTitle className="flex items-center gap-2 text-base"><Wrench className="size-4" />模型可见工具</CardTitle><CardDescription>工具名与 schema 保持稳定；开关由服务器策略真正执行。</CardDescription></div><div className="flex gap-2"><Button size="sm" variant="outline" onClick={() => policyMutation.mutate({ profile: "fast", disabledTools: ["request_analysis"] })}>快速只读</Button><Button size="sm" variant="outline" onClick={() => policyMutation.mutate({ profile: "research", disabledTools: [] })}>科研讨论</Button></div></div></CardHeader>
              <CardContent className="grid gap-2 md:grid-cols-2">{data.toolPolicy.tools.map((tool) => <div key={tool.name} className="flex items-start justify-between gap-3 rounded-lg border p-3"><div className="min-w-0"><div className="flex flex-wrap items-center gap-2"><code className="text-xs font-semibold">{tool.name}</code><Badge variant="outline">{TOOL_LATENCY[tool.latencyClass]}</Badge>{tool.computeCost === "codex_tokens" ? <Badge variant="secondary">消耗 Codex 额度</Badge> : null}</div><p className="mt-2 text-xs text-muted-foreground">{tool.description}</p>{tool.dependencies.length ? <p className="mt-1 text-[11px] text-muted-foreground">依赖：{tool.dependencies.join(", ")}</p> : null}</div><Switch checked={tool.enabled} disabled={["list_workspaces", "get_job"].includes(tool.name) || policyMutation.isPending} onCheckedChange={(checked) => setToolEnabled(tool.name, checked)} aria-label={`${tool.name} 启用状态`} /></div>)}</CardContent>
            </Card>

            <Card>
              <CardHeader><CardTitle className="flex items-center gap-2 text-base"><Bot className="size-4" />Analysis Job Center</CardTitle><CardDescription>跟踪异步 Codex 分析，不再依赖网页反复猜测状态。</CardDescription></CardHeader>
              <CardContent className="max-h-[440px] space-y-2 overflow-y-auto">{data.jobs.jobs.length ? data.jobs.jobs.map((job) => <div key={job.jobId} className="rounded-lg border p-3"><div className="flex items-center justify-between gap-2"><code className="truncate text-xs">{job.jobId}</code><Badge variant={job.status === "completed" ? "secondary" : job.status === "failed" ? "destructive" : "outline"}>{job.status}</Badge></div><p className="mt-2 text-xs">{job.progress || "没有进度信息"}</p><div className="mt-2 flex flex-wrap gap-x-3 text-[11px] text-muted-foreground"><span>{job.workspaceId}</span><span>{formatTime(job.updatedAt)}</span>{job.errorType ? <span>{job.errorType}</span> : null}</div></div>) : <p className="text-sm text-muted-foreground">尚无分析任务。</p>}</CardContent>
            </Card>
          </section>

          <Card>
            <CardHeader><div className="flex items-center justify-between"><div><CardTitle className="flex items-center gap-2 text-base"><Activity className="size-4" />最近工具调用</CardTitle><CardDescription>审计元数据不保存问题正文、证据内容或原始 Codex 会话。</CardDescription></div><Badge variant="outline">最近 {data.activity.records.length} 条</Badge></div></CardHeader>
            <CardContent><div className="overflow-x-auto"><table className="w-full text-left text-sm"><thead className="border-b text-xs text-muted-foreground"><tr><th className="pb-2 font-medium">时间</th><th className="pb-2 font-medium">工具</th><th className="pb-2 font-medium">工作区</th><th className="pb-2 font-medium">状态/结果</th><th className="pb-2 font-medium">Job</th></tr></thead><tbody>{data.activity.records.map((record, index) => <tr key={`${record.timestamp}-${index}`} className="border-b last:border-0"><td className="py-2 pr-4 whitespace-nowrap text-xs">{formatTime(record.timestamp)}</td><td className="py-2 pr-4"><code className="text-xs">{record.event}</code></td><td className="py-2 pr-4 text-xs">{record.workspaceId || "-"}</td><td className="py-2 pr-4 text-xs">{record.status || (record.resultCount != null ? `${record.resultCount} 条` : record.experimentCount != null ? `${record.experimentCount} 个实验` : "成功")}</td><td className="py-2 text-xs">{record.jobId || "-"}</td></tr>)}</tbody></table></div></CardContent>
          </Card>
        </>
      ) : <Card><CardContent className="p-8 text-sm text-muted-foreground">正在读取 LabContext 控制面…</CardContent></Card>}

      {contextMenu ? <div className="fixed z-50 min-w-64 rounded-lg border bg-popover p-1 text-sm shadow-lg" style={{ left: contextMenu.x, top: contextMenu.y }} onClick={(event) => event.stopPropagation()}><button className="flex w-full items-center gap-2 whitespace-nowrap rounded px-3 py-2 text-left hover:bg-muted" onClick={() => { openResearchMap(contextMenu.workspace.workspaceId); setContextMenu(null); }}><BrainCircuit className="size-4" />打开研究图</button><button className="flex w-full items-center gap-2 whitespace-nowrap rounded px-3 py-2 text-left hover:bg-muted disabled:opacity-50" disabled={contextMenu.workspace.isDefault} onClick={() => { defaultMutation.mutate(contextMenu.workspace.workspaceId); setContextMenu(null); }}><Star className="size-4" />设为 ChatGPT 默认</button><button className="flex w-full items-center gap-2 whitespace-nowrap rounded px-3 py-2 text-left hover:bg-muted" onClick={() => { refreshMutation.mutate(contextMenu.workspace.workspaceId); setContextMenu(null); }}><RefreshCw className="size-4" />刷新索引与覆盖</button><button className="flex w-full items-center gap-2 whitespace-nowrap rounded px-3 py-2 text-left hover:bg-muted" onClick={() => { generateOverviewMutation.mutate({ workspaceId: contextMenu.workspace.workspaceId, refresh: true }); setContextMenu(null); }}><Sparkles className="size-4" />用 Codex 重新生成概述</button><div className="my-1 border-t" /><button className="flex w-full items-center gap-2 whitespace-nowrap rounded px-3 py-2 text-left text-destructive hover:bg-destructive/10 disabled:opacity-50" disabled={contextMenu.workspace.isDefault} onClick={() => { setDeleteTarget(contextMenu.workspace); setContextMenu(null); }}><Trash2 className="size-4" />删除工作区注册</button></div> : null}

      <Dialog open={workspaceDialog} onOpenChange={setWorkspaceDialog}>
        <DialogContent>
          <DialogHeader><DialogTitle>添加科研工作区</DialogTitle><DialogDescription>只需要名称和服务器目录。LabContext 会自动生成 ID、识别项目内容，并创建可编辑的 `.labcontext/context.yaml` 项目概述。</DialogDescription></DialogHeader>
          <div className="grid gap-4 py-2"><div className="grid gap-2"><Label htmlFor="workspace-name">工作区名称</Label><Input id="workspace-name" value={workspaceForm.name} onChange={(event) => setWorkspaceForm((value) => ({ ...value, name: event.target.value }))} placeholder="Example research project" /></div><div className="grid gap-2"><Label htmlFor="workspace-root">服务器绝对路径</Label><Input id="workspace-root" value={workspaceForm.root} onChange={(event) => setWorkspaceForm((value) => ({ ...value, root: event.target.value }))} placeholder="/srv/research/project" /></div></div>
          <DialogFooter><Button variant="outline" onClick={() => setWorkspaceDialog(false)}>取消</Button><Button disabled={workspaceMutation.isPending || !workspaceForm.name.trim() || !workspaceForm.root.trim()} onClick={() => workspaceMutation.mutate()}>{workspaceMutation.isPending ? "正在识别项目…" : "添加并自动配置"}</Button></DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(overviewEditor)} onOpenChange={(open) => { if (!open) setOverviewEditor(null); }}>
        <DialogContent>
          <DialogHeader><DialogTitle>编辑工作区概述</DialogTitle><DialogDescription>这段内容会写入工作区的 context.yaml，并作为 ChatGPT 理解项目的优先背景。建议写清研究目标、当前阶段和主要对象。</DialogDescription></DialogHeader>
          {overviewEditor ? <div className="grid gap-2 py-2"><Label htmlFor="workspace-overview">{overviewEditor.name}</Label><Textarea id="workspace-overview" className="min-h-36" maxLength={1200} value={overviewEditor.overview} onChange={(event) => setOverviewEditor((value) => value ? { ...value, overview: event.target.value } : value)} /><p className="text-right text-xs text-muted-foreground">{overviewEditor.overview.length}/1200</p></div> : null}
          <DialogFooter className="sm:justify-between"><Button variant="outline" disabled={generateOverviewMutation.isPending} onClick={() => overviewEditor && generateOverviewMutation.mutate({ workspaceId: overviewEditor.workspaceId, refresh: true })}><Sparkles />{generateOverviewMutation.isPending ? "正在启动…" : "让 Codex 重新生成"}</Button><div className="flex gap-2"><Button variant="outline" onClick={() => setOverviewEditor(null)}>取消</Button><Button disabled={!overviewEditor?.overview.trim() || overviewMutation.isPending} onClick={() => overviewEditor && overviewMutation.mutate({ workspaceId: overviewEditor.workspaceId, overview: overviewEditor.overview })}>{overviewMutation.isPending ? "正在保存…" : "保存概述"}</Button></div></DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(deleteTarget)} onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}>
        <DialogContent>
          <DialogHeader><DialogTitle>删除工作区注册？</DialogTitle><DialogDescription>将从 LabContext 中移除“{deleteTarget?.name}”。服务器上的项目目录、代码、实验结果和 context.yaml 都不会被删除，之后仍可用相同路径重新添加。</DialogDescription></DialogHeader>
          <DialogFooter><Button variant="outline" onClick={() => setDeleteTarget(null)}>取消</Button><Button variant="destructive" disabled={!deleteTarget || deleteTarget.isDefault || deleteMutation.isPending} onClick={() => deleteTarget && deleteMutation.mutate(deleteTarget.workspaceId)}><Trash2 />{deleteMutation.isPending ? "正在移除…" : "仅移除注册"}</Button></DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(testResult)} onOpenChange={(open) => { if (!open) setTestResult(null); }}>
        <DialogContent className="md:max-w-3xl">
          <DialogHeader><DialogTitle>模型可见结果验证</DialogTitle><DialogDescription>这是 LabContext 工具实际返回给模型的结构化内容，不是控制台重新生成的摘要。</DialogDescription></DialogHeader>
          {testResult ? <div className="grid min-h-0 gap-3"><div className="flex flex-wrap gap-2 text-xs"><Badge variant="outline">{testResult.tool}</Badge><Badge variant="secondary">{testResult.elapsedMs} ms</Badge><Badge variant="secondary">{formatBytes(testResult.responseBytes)}</Badge><span className="text-muted-foreground">{formatTime(testResult.testedAt)}</span></div><pre className="max-h-[60vh] overflow-auto rounded-lg bg-muted p-4 text-xs">{JSON.stringify(testResult.result, null, 2)}</pre></div> : null}
        </DialogContent>
      </Dialog>
    </main>
  );
}
