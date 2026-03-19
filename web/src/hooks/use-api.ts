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

export const useAgents = () =>
  useQuery({
    queryKey: ["agents"],
    queryFn: api.listAgents,
    refetchInterval: 5_000,
  });

export const useAgent = (id: string) =>
  useQuery({
    queryKey: ["agent", id],
    queryFn: () => api.getAgent(id),
    enabled: Boolean(id),
  });

export const useAgentStatus = (id: string) =>
  useQuery({
    queryKey: ["agent-status", id],
    queryFn: () => api.getAgentStatus(id),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });

export const useAgentSkills = (id: string) =>
  useQuery({
    queryKey: ["agent-skills", id],
    queryFn: () => api.getAgentSkills(id),
    enabled: Boolean(id),
  });

export const useCompany = () =>
  useQuery({ queryKey: ["company"], queryFn: api.getCompany });

export const useOrgTree = () =>
  useQuery({ queryKey: ["org-tree"], queryFn: api.getOrgTree });

export const useOrgAgents = () =>
  useQuery({ queryKey: ["org-agents"], queryFn: api.listOrgAgents });

export const useSystemStatus = () =>
  useQuery({
    queryKey: ["system-status"],
    queryFn: api.getSystemStatus,
    refetchInterval: 5_000,
  });

export const useSystemHealth = () =>
  useQuery({
    queryKey: ["system-health"],
    queryFn: api.getSystemHealth,
    refetchInterval: 10_000,
  });

export const usePlugins = () =>
  useQuery({
    queryKey: ["plugins"],
    queryFn: api.getPlugins,
    refetchInterval: 5_000,
  });

export const usePlugin = (id?: string) =>
  useQuery({
    queryKey: ["plugin", id],
    queryFn: () => api.getPlugin(id as string),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });

export const useEnablePlugin = () =>
  useMutation({
    mutationFn: (id: string) => api.enablePlugin(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["plugin", id] });
      queryClient.invalidateQueries({ queryKey: ["system-status"] });
      queryClient.invalidateQueries({ queryKey: ["system-health"] });
    },
  });

export const useDisablePlugin = () =>
  useMutation({
    mutationFn: (id: string) => api.disablePlugin(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["plugin", id] });
      queryClient.invalidateQueries({ queryKey: ["system-status"] });
      queryClient.invalidateQueries({ queryKey: ["system-health"] });
    },
  });

export const useReloadPlugin = () =>
  useMutation({
    mutationFn: (id: string) => api.reloadPlugin(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["plugin", id] });
      queryClient.invalidateQueries({ queryKey: ["system-status"] });
      queryClient.invalidateQueries({ queryKey: ["system-health"] });
    },
  });

export const useInstallPlugin = () =>
  useMutation({
    mutationFn: (sourcePath: string) => api.installPlugin(sourcePath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["system-status"] });
      queryClient.invalidateQueries({ queryKey: ["system-health"] });
    },
  });

export const useUninstallPlugin = () =>
  useMutation({
    mutationFn: (id: string) => api.uninstallPlugin(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["plugin", id] });
      queryClient.invalidateQueries({ queryKey: ["system-status"] });
      queryClient.invalidateQueries({ queryKey: ["system-health"] });
    },
  });

export const useTrustPluginKey = () =>
  useMutation({
    mutationFn: ({ alias, publicKeyPath }: { alias: string; publicKeyPath: string }) =>
      api.trustPluginKey(alias, publicKeyPath),
  });

export const useSignPlugin = () =>
  useMutation({
    mutationFn: ({ id, privateKeyPath }: { id: string; privateKeyPath: string }) =>
      api.signPlugin(id, privateKeyPath),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ["plugins"] });
      queryClient.invalidateQueries({ queryKey: ["plugin", variables.id] });
    },
  });

export const usePluginLogs = (id?: string) =>
  useQuery({
    queryKey: ["plugin-logs", id],
    queryFn: () => api.getPluginLogs(id as string),
    enabled: Boolean(id),
    refetchInterval: 5_000,
  });

export const usePluginDoctor = () =>
  useQuery({
    queryKey: ["plugin-doctor"],
    queryFn: api.getPluginDoctor,
    refetchInterval: 10_000,
  });

export const useConversations = () =>
  useQuery({
    queryKey: ["conversations"],
    queryFn: api.listConversations,
    refetchInterval: 5_000,
  });

export const useConversation = (id: string) =>
  useQuery({
    queryKey: ["conversation", id],
    queryFn: () => api.getConversation(id),
    enabled: Boolean(id),
  });

export const useMessages = (id: string) =>
  useQuery({
    queryKey: ["messages", id],
    queryFn: () => api.listMessages(id),
    enabled: Boolean(id),
    refetchInterval: 2_000,
  });
