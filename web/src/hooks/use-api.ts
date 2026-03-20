import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../lib/api";
import { queryClient } from "../query-client";

export function useDashboardData() {
  const system = useQuery({
    queryKey: ["system-status"],
    queryFn: api.getSystemStatus,
    refetchInterval: 5_000,
  });
  const health = useQuery({
    queryKey: ["system-health"],
    queryFn: api.getSystemHealth,
    refetchInterval: 10_000,
  });
  const agents = useQuery({
    queryKey: ["agents"],
    queryFn: api.listAgents,
    refetchInterval: 5_000,
  });

  return { system, health, agents };
}

export function useAgents() {
  return useQuery({
    queryKey: ["agents"],
    queryFn: api.listAgents,
    refetchInterval: 5_000,
  });
}

export function useAgent(id?: string | null) {
  return useQuery({
    queryKey: ["agent", id],
    queryFn: () => api.getAgent(id!),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });
}

export function useAgentStatus(id?: string | null) {
  return useQuery({
    queryKey: ["agent-status", id],
    queryFn: () => api.getAgentStatus(id!),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });
}

export function useAgentSkills(id?: string | null) {
  return useQuery({
    queryKey: ["agent-skills", id],
    queryFn: () => api.getAgentSkills(id!),
    enabled: Boolean(id),
    refetchInterval: 30_000,
  });
}

export function useOrgTree() {
  return useQuery({
    queryKey: ["org-tree"],
    queryFn: api.getOrgTree,
    refetchInterval: 10_000,
  });
}

export function useOrgAgents() {
  return useQuery({
    queryKey: ["org-agents"],
    queryFn: api.listOrgAgents,
    refetchInterval: 10_000,
  });
}

export function useSystemStatus() {
  return useQuery({
    queryKey: ["system-status"],
    queryFn: api.getSystemStatus,
    refetchInterval: 5_000,
  });
}

export function useSystemHealth() {
  return useQuery({
    queryKey: ["system-health"],
    queryFn: api.getSystemHealth,
    refetchInterval: 10_000,
  });
}

export function usePlugins() {
  return useQuery({
    queryKey: ["plugins"],
    queryFn: api.getPlugins,
    refetchInterval: 10_000,
  });
}

export function usePluginLogs(id?: string | null) {
  return useQuery({
    queryKey: ["plugin-logs", id],
    queryFn: () => api.getPluginLogs(id!),
    enabled: Boolean(id),
    refetchInterval: 10_000,
  });
}

export function usePluginDoctor() {
  return useQuery({
    queryKey: ["plugin-doctor"],
    queryFn: api.getPluginDoctor,
    refetchInterval: 10_000,
  });
}

function invalidatePluginQueries() {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: ["plugins"] }),
    queryClient.invalidateQueries({ queryKey: ["plugin-doctor"] }),
    queryClient.invalidateQueries({ queryKey: ["plugin-logs"] }),
    queryClient.invalidateQueries({ queryKey: ["system-status"] }),
    queryClient.invalidateQueries({ queryKey: ["system-health"] }),
  ]);
}

export function useEnablePlugin() {
  return useMutation({
    mutationFn: (id: string) => api.enablePlugin(id),
    onSuccess: invalidatePluginQueries,
  });
}

export function useDisablePlugin() {
  return useMutation({
    mutationFn: (id: string) => api.disablePlugin(id),
    onSuccess: invalidatePluginQueries,
  });
}

export function useReloadPlugin() {
  return useMutation({
    mutationFn: (id: string) => api.reloadPlugin(id),
    onSuccess: invalidatePluginQueries,
  });
}

export function useInstallPlugin() {
  return useMutation({
    mutationFn: (sourcePath: string) => api.installPlugin(sourcePath),
    onSuccess: invalidatePluginQueries,
  });
}

export function useUninstallPlugin() {
  return useMutation({
    mutationFn: (id: string) => api.uninstallPlugin(id),
    onSuccess: invalidatePluginQueries,
  });
}

export function useTrustPluginKey() {
  return useMutation({
    mutationFn: ({ alias, publicKeyPath }: { alias: string; publicKeyPath: string }) =>
      api.trustPluginKey(alias, publicKeyPath),
    onSuccess: invalidatePluginQueries,
  });
}

export function useSignPlugin() {
  return useMutation({
    mutationFn: ({ id, privateKeyPath }: { id: string; privateKeyPath: string }) =>
      api.signPlugin(id, privateKeyPath),
    onSuccess: invalidatePluginQueries,
  });
}

export function useConversations() {
  return useQuery({
    queryKey: ["conversations"],
    queryFn: api.listConversations,
    refetchInterval: 5_000,
  });
}

export function useConversation(id?: string | null) {
  return useQuery({
    queryKey: ["conversation", id],
    queryFn: () => api.getConversation(id!),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });
}

export function useMessages(id?: string | null) {
  return useQuery({
    queryKey: ["messages", id],
    queryFn: () => api.listMessages(id!),
    enabled: Boolean(id),
    refetchInterval: 3_000,
  });
}
