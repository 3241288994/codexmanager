"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Background, Controls, Handle, MarkerType, MiniMap, Position, ReactFlow,
  useEdgesState, useNodesState, type Connection, type Edge, type Node, type NodeProps,
} from "@xyflow/react";
import ELK from "elkjs/lib/elk.bundled.js";
import {
  AlertCircle, Beaker, Bot, BrainCircuit, Check, FlaskConical, GitBranch,
  Lightbulb, LoaderCircle, LocateFixed, Pencil, Plus, RefreshCw,
  Save, ShieldAlert, Sparkles, Target, Trash2, X,
} from "lucide-react";
import { toast } from "sonner";
import "@xyflow/react/dist/style.css";

import { labContextClient } from "@/lib/api/labcontext-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import type {
  LabContextWorkspace, ResearchMapEdge, ResearchMapNode, ResearchMapNodeType,
  ResearchMapPatch, ResearchMapProposal,
} from "@/types/labcontext";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

const elk = new ELK();

const NODE_META: Record<ResearchMapNodeType, { label: string; color: string; icon: typeof Lightbulb }> = {
  core_idea: { label: "核心想法", color: "#8b5cf6", icon: Lightbulb },
  claim: { label: "研究主张", color: "#2563eb", icon: BrainCircuit },
  branch: { label: "探索分支", color: "#0891b2", icon: GitBranch },
  current_target: { label: "当前目标", color: "#ea580c", icon: Target },
  experiment: { label: "实验", color: "#059669", icon: FlaskConical },
  evidence: { label: "证据", color: "#16a34a", icon: Beaker },
  decision: { label: "决策", color: "#7c3aed", icon: Check },
  risk: { label: "风险 / 阻塞", color: "#dc2626", icon: ShieldAlert },
};

const RELATION_LABELS: Record<string, string> = {
  decomposes_into: "拆分为", tests: "检验", supports: "支持", contradicts: "反驳",
  motivated_by: "由此驱动", blocked_by: "受阻于", supersedes: "取代", derived_from: "源自",
};

type MapNodeData = ResearchMapNode & Record<string, unknown>;
type FlowNode = Node<MapNodeData, "research">;

function ResearchNodeCard({ data, selected }: NodeProps<FlowNode>) {
  const meta = NODE_META[data.type];
  const Icon = meta.icon;
  return (
    <div
      className={`w-[260px] rounded-2xl border bg-card/95 p-3.5 text-card-foreground shadow-sm backdrop-blur transition-shadow ${selected ? "ring-2 ring-primary/60 shadow-lg" : "hover:shadow-md"}`}
      style={{ borderTop: `3px solid ${meta.color}` }}
    >
      <Handle type="target" position={Position.Left} className="!size-2.5 !border-2 !border-background" style={{ background: meta.color }} />
      <div className="flex items-start gap-2.5">
        <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-xl text-white" style={{ background: meta.color }}><Icon className="size-4" /></div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2"><span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">{meta.label}</span><Badge variant="outline" className="h-5 max-w-24 truncate px-1.5 text-[9px]">{data.status}</Badge></div>
          <p className="mt-1 line-clamp-2 text-sm font-semibold leading-5">{data.title}</p>
          {data.summary ? <p className="mt-1.5 line-clamp-3 text-[11px] leading-4 text-muted-foreground">{data.summary}</p> : null}
          {data.sourceRefs.length ? <p className="mt-2 text-[10px] text-muted-foreground">{data.sourceRefs.length} 条证据引用</p> : null}
        </div>
      </div>
      <Handle type="source" position={Position.Right} className="!size-2.5 !border-2 !border-background" style={{ background: meta.color }} />
    </div>
  );
}

const nodeTypes = { research: ResearchNodeCard };

function toFlowEdges(edges: ResearchMapEdge[]): Edge[] {
  return edges.map((edge) => ({
    id: edge.id, source: edge.source, target: edge.target,
    label: edge.label || RELATION_LABELS[edge.relation] || edge.relation,
    type: "smoothstep", markerEnd: { type: MarkerType.ArrowClosed, width: 14, height: 14 },
    style: { stroke: "var(--muted-foreground)", strokeOpacity: 0.5 },
    labelStyle: { fontSize: 10, fill: "var(--muted-foreground)" },
    labelBgStyle: { fill: "var(--background)", fillOpacity: 0.9 },
  }));
}

async function autoLayout(nodes: ResearchMapNode[], edges: ResearchMapEdge[]): Promise<FlowNode[]> {
  const graph = await elk.layout({
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered", "elk.direction": "RIGHT", "elk.spacing.nodeNode": "42",
      "elk.layered.spacing.nodeNodeBetweenLayers": "90", "elk.edgeRouting": "ORTHOGONAL",
    },
    children: nodes.map((node) => ({ id: node.id, width: 260, height: node.summary ? 150 : 112 })),
    edges: edges.map((edge) => ({ id: edge.id, sources: [edge.source], targets: [edge.target] })),
  });
  const positions = new Map((graph.children || []).map((item) => [item.id, { x: item.x || 0, y: item.y || 0 }]));
  return nodes.map((node) => ({ id: node.id, type: "research", position: positions.get(node.id) || { x: 0, y: 0 }, data: node as MapNodeData }));
}

function proposalTitle(proposal: ResearchMapProposal): string {
  if (proposal.status === "generating") return "Codex 正在审视研究图";
  if (proposal.status === "failed") return "Codex 审阅失败";
  return proposal.summary || proposal.patch?.summary || "研究图更新提案";
}

export function ResearchMapPanel({ workspace }: { workspace: LabContextWorkspace }) {
  const queryClient = useQueryClient();
  const queryKey = ["labcontext", "research-map", workspace.workspaceId];
  const [view, setView] = useState<"focus" | "all" | "history">("focus");
  const [nodes, setNodes, onNodesChange] = useNodesState<FlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [editor, setEditor] = useState<Partial<ResearchMapNode> | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [proposal, setProposal] = useState<ResearchMapProposal | null>(null);
  const [layoutDirty, setLayoutDirty] = useState(false);

  const mapQuery = useQuery({
    queryKey,
    queryFn: () => labContextClient.getResearchMap(workspace.workspaceId),
    refetchInterval: (query) => query.state.data?.proposals.some((item) => item.status === "generating") ? 5_000 : false,
  });
  const bundle = mapQuery.data;

  const visibleIds = useMemo(() => {
    if (!bundle || view === "all" || view === "history") return null;
    const capsule = bundle.focusCapsule;
    const ids = new Set<string>();
    [capsule.coreIdea, capsule.activeClaim, capsule.currentTarget, capsule.nextExperiment].forEach((item) => { if (item) ids.add(item.id); });
    capsule.blockers.forEach((item) => ids.add(item.id));
    capsule.relatedNodes.forEach((item) => ids.add(item.id));
    return ids;
  }, [bundle, view]);

  useEffect(() => {
    if (!bundle) return;
    const semanticNodes = visibleIds ? bundle.researchMap.nodes.filter((node) => visibleIds.has(node.id)) : bundle.researchMap.nodes;
    const semanticIds = new Set(semanticNodes.map((node) => node.id));
    const semanticEdges = bundle.researchMap.edges.filter((edge) => semanticIds.has(edge.source) && semanticIds.has(edge.target));
    const positions = new Map(bundle.layout.nodes.map((item) => [item.id, { x: item.x, y: item.y }]));
    if (semanticNodes.every((node) => positions.has(node.id))) {
      setNodes(semanticNodes.map((node) => ({ id: node.id, type: "research", position: positions.get(node.id)!, data: node as MapNodeData })));
    } else {
      void autoLayout(semanticNodes, semanticEdges).then(setNodes);
    }
    setEdges(toFlowEdges(semanticEdges));
    setLayoutDirty(false);
  }, [bundle, setEdges, setNodes, visibleIds]);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey }),
      queryClient.invalidateQueries({ queryKey: ["labcontext", "overview"] }),
    ]);
  };
  const initializeMutation = useMutation({
    mutationFn: () => labContextClient.initializeResearchMap(workspace.workspaceId),
    onSuccess: async () => { await invalidate(); toast.success("已根据项目事实初始化研究图，请检查节点内容"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const patchMutation = useMutation({
    mutationFn: (patch: ResearchMapPatch) => labContextClient.applyResearchMapPatch(workspace.workspaceId, patch),
    onSuccess: async () => { setEditor(null); await invalidate(); toast.success("研究图已更新"); },
    onError: (error) => { toast.error(getAppErrorMessage(error)); void mapQuery.refetch(); },
  });
  const layoutMutation = useMutation({
    mutationFn: () => labContextClient.saveResearchMapLayout(workspace.workspaceId, {
      nodes: nodes.map((node) => ({ id: node.id, x: node.position.x, y: node.position.y, collapsed: false })),
      viewport: bundle?.layout.viewport || { x: 0, y: 0, zoom: 1 },
    }),
    onSuccess: async () => { setLayoutDirty(false); await invalidate(); toast.success("画布布局已保存"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const reviewMutation = useMutation({
    mutationFn: () => labContextClient.reviewResearchMap(workspace.workspaceId, true),
    onSuccess: async (result) => {
      await invalidate();
      toast.success(result.status === "queued" ? "已通知该工作区的最新 Codex 会话审视研究图" : "已启动独立 Codex 审阅");
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const proposalMutation = useMutation({
    mutationFn: ({ proposalId, action }: { proposalId: string; action: "apply" | "reject" }) => labContextClient.researchMapProposalAction(workspace.workspaceId, proposalId, action),
    onSuccess: async (_, variables) => { setProposal(null); await invalidate(); toast.success(variables.action === "apply" ? "Codex 提案已应用" : "提案已拒绝，研究事实未改变"); },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const applyOperations = (summary: string, operations: ResearchMapPatch["operations"]) => {
    if (!bundle) return;
    patchMutation.mutate({ baseRevision: bundle.researchMap.revision, summary, operations });
  };
  const saveEditor = () => {
    if (!editor?.title?.trim() || !bundle) return;
    if (isNew) {
      const id = `${editor.type || "branch"}:${crypto.randomUUID().slice(0, 8)}`;
      applyOperations(`添加“${editor.title}”`, [{ op: "add_node", node: { id, type: editor.type || "branch", title: editor.title, summary: editor.summary || "", status: editor.status || "active", authority: "user_reviewed", origin: "user", source_refs: [], locked_fields: [], metadata: {} } }]);
    } else if (editor.id) {
      applyOperations(`更新“${editor.title}”`, [{ op: "update_node", node_id: editor.id, changes: { title: editor.title, summary: editor.summary || "", status: editor.status || "active", authority: "user_reviewed" } }]);
    }
  };
  const connect = useCallback((connection: Connection) => {
    if (!connection.source || !connection.target || !bundle) return;
    applyOperations("连接两个研究节点", [{ op: "add_edge", edge: { id: `edge:${crypto.randomUUID().slice(0, 8)}`, source: connection.source, target: connection.target, relation: "decomposes_into", label: "", origin: "user", source_refs: [] } }]);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bundle]);
  const relayout = async () => {
    if (!bundle) return;
    const nodeIds = new Set(nodes.map((node) => node.id));
    const laidOut = await autoLayout(bundle.researchMap.nodes.filter((node) => nodeIds.has(node.id)), bundle.researchMap.edges.filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)));
    setNodes(laidOut); setLayoutDirty(true);
  };

  const selectedSemantic = bundle?.researchMap.nodes.find((node) => node.id === selectedNodeId) || null;
  const pending = bundle?.proposals.filter((item) => ["pending", "generating", "failed"].includes(item.status)) || [];
  const focus = bundle?.focusCapsule;

  return (
    <Card id="research-map" className="overflow-hidden border-primary/20">
      <CardHeader className="border-b bg-gradient-to-r from-primary/[0.06] via-background to-background pb-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <CardTitle className="flex items-center gap-2 text-base"><BrainCircuit className="size-4 text-primary" />研究图 · {workspace.name}</CardTitle>
            <CardDescription className="mt-1">把核心想法、当前目标、实验、证据与风险放在同一张可维护的图中。</CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={() => reviewMutation.mutate()} disabled={!bundle?.researchMap.nodes.length || reviewMutation.isPending}><Bot />{reviewMutation.isPending ? "正在通知…" : "让最新 Codex 审视"}</Button>
            <Button size="sm" onClick={() => { setIsNew(true); setEditor({ type: "branch", title: "", summary: "", status: "active" }); }} disabled={!bundle?.researchMap.nodes.length}><Plus />添加节点</Button>
          </div>
        </div>
        {focus?.status === "ready" ? (
          <div className="mt-3 grid gap-2 sm:grid-cols-3">
            <div className="rounded-xl border bg-background/80 p-3"><p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">核心问题</p><p className="mt-1 line-clamp-2 text-sm font-medium">{focus.coreIdea?.title || "未设置"}</p></div>
            <div className="rounded-xl border border-orange-500/20 bg-orange-500/[0.04] p-3"><p className="text-[10px] font-semibold uppercase tracking-wider text-orange-600">当前目标</p><p className="mt-1 line-clamp-2 text-sm font-medium">{focus.currentTarget?.title || "未设置"}</p></div>
            <div className="rounded-xl border border-red-500/20 bg-red-500/[0.04] p-3"><p className="text-[10px] font-semibold uppercase tracking-wider text-red-600">开放风险</p><p className="mt-1 text-sm font-medium">{focus.counts.openRisks} 项 · {focus.blockers[0]?.title || "暂无明确阻塞"}</p></div>
          </div>
        ) : null}
      </CardHeader>
      <CardContent className="p-0">
        {mapQuery.isLoading ? <div className="flex h-[520px] items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="size-4 animate-spin" />读取研究图…</div> : null}
        {mapQuery.isError ? <div className="flex h-64 flex-col items-center justify-center gap-3 p-6 text-center"><AlertCircle className="size-7 text-destructive" /><p className="text-sm">{getAppErrorMessage(mapQuery.error)}</p><Button variant="outline" onClick={() => mapQuery.refetch()}><RefreshCw />重试</Button></div> : null}
        {bundle && !bundle.researchMap.nodes.length ? (
          <div className="flex min-h-[360px] flex-col items-center justify-center gap-4 p-8 text-center">
            <div className="flex size-14 items-center justify-center rounded-2xl bg-primary/10"><Sparkles className="size-7 text-primary" /></div>
            <div><h3 className="font-semibold">这个项目还没有研究图</h3><p className="mt-1 max-w-lg text-sm text-muted-foreground">先用 context.yaml、结构化研究状态和实验索引生成一版可核验草图；初始化不消耗 Codex token。</p></div>
            <Button onClick={() => initializeMutation.mutate()} disabled={initializeMutation.isPending}>{initializeMutation.isPending ? <LoaderCircle className="animate-spin" /> : <Sparkles />}{initializeMutation.isPending ? "正在初始化…" : "自动初始化研究图"}</Button>
          </div>
        ) : null}
        {bundle?.researchMap.nodes.length ? (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2 border-b px-4 py-2.5">
              <div className="flex items-center rounded-lg bg-muted p-1">
                <Button size="sm" variant={view === "focus" ? "secondary" : "ghost"} className="h-7" onClick={() => setView("focus")}><LocateFixed />当前焦点</Button>
                <Button size="sm" variant={view === "all" ? "secondary" : "ghost"} className="h-7" onClick={() => setView("all")}><GitBranch />完整研究图</Button>
                <Button size="sm" variant={view === "history" ? "secondary" : "ghost"} className="h-7" onClick={() => setView("history")}><RefreshCw />变更记录</Button>
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>v{bundle.researchMap.revision} · {bundle.researchMap.nodes.length} 节点 · {bundle.researchMap.edges.length} 关系</span>
                <Button size="sm" variant="ghost" className="h-7" onClick={relayout}><RefreshCw />自动排版</Button>
                <Button size="sm" variant={layoutDirty ? "default" : "outline"} className="h-7" onClick={() => layoutMutation.mutate()} disabled={!layoutDirty || layoutMutation.isPending}><Save />保存布局</Button>
              </div>
            </div>
            {view !== "history" ? <div className="grid min-h-[560px] lg:grid-cols-[minmax(0,1fr)_300px]">
              <div className="h-[560px] min-w-0 bg-[radial-gradient(circle_at_center,var(--muted)_0,transparent_70%)]">
                <ReactFlow<FlowNode, Edge>
                  nodes={nodes} edges={edges} nodeTypes={nodeTypes} onNodesChange={(changes) => { onNodesChange(changes); if (changes.some((item) => item.type === "position")) setLayoutDirty(true); }} onEdgesChange={onEdgesChange}
                  onConnect={connect} onNodeClick={(_, node) => setSelectedNodeId(node.id)} onPaneClick={() => setSelectedNodeId(null)} fitView fitViewOptions={{ padding: 0.2 }} minZoom={0.25} maxZoom={1.6}
                >
                  <Background gap={22} size={1} color="var(--border)" /><MiniMap pannable zoomable nodeColor={(node) => NODE_META[(node.data as MapNodeData).type]?.color || "#94a3b8"} /><Controls position="bottom-left" />
                </ReactFlow>
              </div>
              <aside className="border-l bg-muted/20 p-4">
                {selectedSemantic ? (
                  <div className="space-y-4">
                    <div><Badge variant="outline">{NODE_META[selectedSemantic.type].label}</Badge><h3 className="mt-2 font-semibold leading-6">{selectedSemantic.title}</h3><p className="mt-2 text-sm leading-6 text-muted-foreground">{selectedSemantic.summary || "暂无详细说明"}</p></div>
                    <div className="grid grid-cols-2 gap-2 text-xs"><div className="rounded-lg border bg-background p-2"><p className="text-muted-foreground">状态</p><p className="mt-1 font-medium">{selectedSemantic.status}</p></div><div className="rounded-lg border bg-background p-2"><p className="text-muted-foreground">信息性质</p><p className="mt-1 font-medium">{selectedSemantic.authority}</p></div></div>
                    {selectedSemantic.sourceRefs.length ? <div><p className="text-xs font-medium">证据引用</p><div className="mt-2 space-y-1">{selectedSemantic.sourceRefs.map((ref) => <code key={ref} className="block break-all rounded bg-muted px-2 py-1 text-[10px]">{ref}</code>)}</div></div> : null}
                    <div className="flex gap-2"><Button size="sm" variant="outline" onClick={() => { setIsNew(false); setEditor(selectedSemantic); }}><Pencil />编辑</Button><Button size="sm" variant="ghost" className="text-destructive" onClick={() => applyOperations(`归档“${selectedSemantic.title}”`, [{ op: "archive_node", node_id: selectedSemantic.id }])}><Trash2 />归档</Button></div>
                  </div>
                ) : (
                  <div className="space-y-3"><p className="text-sm font-medium">如何阅读</p><p className="text-xs leading-5 text-muted-foreground">单击节点查看详情并编辑。拖动节点调整版面，从节点右侧连接点拖向另一个节点可建立“拆分为”关系。</p><div className="grid grid-cols-2 gap-2">{Object.entries(NODE_META).map(([key, meta]) => <div key={key} className="flex items-center gap-2 text-xs"><span className="size-2 rounded-full" style={{ background: meta.color }} />{meta.label}</div>)}</div></div>
                )}
              </aside>
            </div> : <div className="min-h-[420px] p-5"><div className="mx-auto max-w-3xl space-y-3">{bundle.events.length ? bundle.events.map((event) => <div key={event.eventId} className="grid grid-cols-[auto_minmax(0,1fr)] gap-3 rounded-xl border bg-background p-4"><div className="mt-1 size-2 rounded-full bg-primary" /><div><div className="flex flex-wrap items-center justify-between gap-2"><p className="text-sm font-medium">{event.summary}</p><Badge variant="outline">v{event.revision}</Badge></div><p className="mt-1 text-xs text-muted-foreground">{event.eventType} · {event.actor} · {new Date(event.timestamp).toLocaleString()}</p></div></div>) : <p className="py-16 text-center text-sm text-muted-foreground">尚无研究图变更记录。</p>}</div></div>}
          </>
        ) : null}

        {pending.length ? (
          <div className="border-t bg-amber-500/[0.035] p-4">
            <div className="mb-3 flex items-center justify-between"><div><p className="text-sm font-semibold">Codex 审阅提案</p><p className="text-xs text-muted-foreground">提案不会自动改变研究事实，需由你确认。</p></div><Badge variant="secondary">{pending.length}</Badge></div>
            <div className="grid gap-2 md:grid-cols-2">{pending.map((item) => <button key={item.proposalId} onClick={() => setProposal(item)} className="rounded-xl border bg-background p-3 text-left transition-colors hover:border-primary/40"><div className="flex items-center justify-between gap-2"><p className="line-clamp-1 text-sm font-medium">{proposalTitle(item)}</p><Badge variant={item.status === "failed" ? "destructive" : "outline"}>{item.status}</Badge></div><p className="mt-2 text-xs text-muted-foreground">{item.sourceKind} · 基于 v{item.baseRevision} · {item.patch?.operations.length || 0} 项变更</p></button>)}</div>
          </div>
        ) : null}
      </CardContent>

      <Dialog open={Boolean(editor)} onOpenChange={(open) => { if (!open) setEditor(null); }}>
        <DialogContent>
          <DialogHeader><DialogTitle>{isNew ? "添加研究节点" : "编辑研究节点"}</DialogTitle><DialogDescription>这里保存的是研究语义，不包含画布坐标。状态与描述会进入 ChatGPT 的工作区背景。</DialogDescription></DialogHeader>
          {editor ? <div className="grid gap-4 py-2"><div className="grid gap-2"><Label>类型</Label><select className="h-9 rounded-lg border bg-background px-3 text-sm" value={editor.type} disabled={!isNew} onChange={(event) => setEditor((value) => ({ ...value, type: event.target.value as ResearchMapNodeType }))}>{Object.entries(NODE_META).map(([key, meta]) => <option key={key} value={key}>{meta.label}</option>)}</select></div><div className="grid gap-2"><Label>标题</Label><Input value={editor.title || ""} maxLength={180} onChange={(event) => setEditor((value) => ({ ...value, title: event.target.value }))} /></div><div className="grid gap-2"><Label>说明</Label><Textarea className="min-h-28" value={editor.summary || ""} maxLength={2000} onChange={(event) => setEditor((value) => ({ ...value, summary: event.target.value }))} /></div><div className="grid gap-2"><Label>状态</Label><Input value={editor.status || "active"} maxLength={64} onChange={(event) => setEditor((value) => ({ ...value, status: event.target.value }))} placeholder="active / planned / resolved" /></div></div> : null}
          <DialogFooter><Button variant="outline" onClick={() => setEditor(null)}>取消</Button><Button onClick={saveEditor} disabled={!editor?.title?.trim() || patchMutation.isPending}>{patchMutation.isPending ? <LoaderCircle className="animate-spin" /> : <Save />}保存</Button></DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(proposal)} onOpenChange={(open) => { if (!open) setProposal(null); }}>
        <DialogContent className="md:max-w-2xl">
          <DialogHeader><DialogTitle>{proposal ? proposalTitle(proposal) : "Codex 提案"}</DialogTitle><DialogDescription>查看 Codex 建议的结构化变更。应用时会再次校验版本和受保护字段。</DialogDescription></DialogHeader>
          {proposal ? <div className="max-h-[55vh] space-y-3 overflow-y-auto py-2">{proposal.error ? <p className="rounded-lg bg-destructive/10 p-3 text-sm text-destructive">{proposal.error}</p> : null}{proposal.status === "generating" ? <div className="flex items-center gap-2 rounded-lg border p-4 text-sm"><LoaderCircle className="size-4 animate-spin" />Codex 正在读取紧凑研究图上下文并生成补丁…</div> : proposal.patch?.operations.map((operation, index) => <div key={index} className="rounded-lg border p-3"><div className="flex items-center gap-2"><Badge variant="outline">{String(operation.op)}</Badge><span className="text-xs text-muted-foreground">变更 {index + 1}</span></div><pre className="mt-2 overflow-x-auto whitespace-pre-wrap break-words text-xs text-muted-foreground">{JSON.stringify(operation, null, 2)}</pre></div>)}</div> : null}
          <DialogFooter className="sm:justify-between"><Button variant="outline" onClick={() => setProposal(null)}>关闭</Button><div className="flex gap-2"><Button variant="outline" disabled={!proposal || proposal.status !== "pending" || proposalMutation.isPending} onClick={() => proposal && proposalMutation.mutate({ proposalId: proposal.proposalId, action: "reject" })}><X />拒绝</Button><Button disabled={!proposal || proposal.status !== "pending" || proposalMutation.isPending} onClick={() => proposal && proposalMutation.mutate({ proposalId: proposal.proposalId, action: "apply" })}><Check />确认应用</Button></div></DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
