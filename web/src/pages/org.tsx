import { useMemo, useState } from "react";
import { Background, Controls, MiniMap, ReactFlow, type Edge, type Node } from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useOrgAgents, useOrgTree } from "../hooks/use-api";
import type { OrgNode } from "../types/api";
import { SectionCard } from "../components/section-card";
import { useI18n } from "../i18n";

function flattenTree(root: OrgNode) {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  const levels = new Map<number, number>();

  const walk = (node: OrgNode, parentId?: string) => {
    const index = levels.get(node.depth) ?? 0;
    levels.set(node.depth, index + 1);
    nodes.push({
      id: node.node_id,
      data: { label: `${node.name}\n${node.title}` },
      position: { x: node.depth * 260, y: index * 130 },
      style: {
        borderRadius: 20,
        border: "1px solid rgba(32,34,31,0.15)",
        background: "#fffaf0",
        padding: 12,
        width: 220,
      },
    });
    if (parentId) {
      edges.push({
        id: `${parentId}-${node.node_id}`,
        source: parentId,
        target: node.node_id,
        animated: false,
      });
    }
    node.children.forEach((child) => walk(child, node.node_id));
  };

  walk(root);
  return { nodes, edges };
}

export function OrgPage() {
  const tree = useOrgTree();
  const agents = useOrgAgents();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { t } = useI18n();
  const isLoading = tree.isLoading || agents.isLoading;
  const loadError = tree.error ?? agents.error;

  const graph = useMemo(() => {
    if (!tree.data?.root) return { nodes: [], edges: [] };
    return flattenTree(tree.data.root);
  }, [tree.data?.root]);

  return (
    <div className="grid gap-5 xl:grid-cols-[1.4fr_0.6fr]">
      <SectionCard title={t("org.title")} subtitle={t("org.subtitle")}>
        {isLoading ? (
          <div className="mb-4 rounded-2xl border border-dashed border-black/10 bg-stone-50 px-4 py-5 text-sm text-graphite/60">
            {t("messages.loading")}
          </div>
        ) : null}
        {loadError ? (
          <div className="mb-4 rounded-2xl border border-amber-300 bg-amber-50 px-4 py-5 text-sm text-amber-900">
            {t("messages.loadError")}
          </div>
        ) : null}
        <div className="h-[720px] overflow-hidden rounded-[24px] border border-black/10 bg-[#faf5ea]">
          <ReactFlow
            nodes={graph.nodes}
            edges={graph.edges}
            fitView
            onNodeClick={(_, node) => setSelectedId(node.id)}
          >
            <MiniMap />
            <Controls />
            <Background gap={18} size={1} />
          </ReactFlow>
          {!graph.nodes.length ? (
            <div className="absolute inset-0 flex items-center justify-center bg-white/90 px-6 text-center text-sm text-graphite/60">
              {t("org.empty")}
            </div>
          ) : null}
        </div>
      </SectionCard>
      <div className="space-y-5">
        <SectionCard title={t("org.list.title")} subtitle={t("org.list.subtitle")}>
          <div className="space-y-2">
            {!agents.data?.agents.length ? (
              <div className="rounded-2xl border border-dashed border-black/10 bg-stone-50 px-4 py-5 text-sm text-graphite/60">
                {t("org.empty")}
              </div>
            ) : null}
            {agents.data?.agents.map((agent) => (
              <button
                key={agent.id}
                className={`w-full rounded-2xl border px-4 py-3 text-left ${
                  selectedId === agent.id ? "border-signal bg-orange-50" : "border-black/10 bg-stone-50"
                }`}
                onClick={() => setSelectedId(agent.id)}
              >
                <p className="font-medium">{agent.name}</p>
                <p className="text-sm text-graphite/65">{agent.role} - {t("org.depth")} {agent.depth}</p>
              </button>
            ))}
          </div>
        </SectionCard>
      </div>
    </div>
  );
}
