import type {
  AgentDetail,
  AgentSkill,
  AgentStatus,
  AgentSummary,
  CompanyInfo,
  Conversation,
  ConversationKind,
  ConversationSummary,
  HealthStatus,
  Message,
  OrgAgent,
  OrgNode,
  PluginDoctorResponse,
  PluginInfo,
  PluginListResponse,
  PluginLogsResponse,
  PluginMutationResponse,
  SystemStatus,
} from "../types/api";
import { getEffectiveApiBaseUrl } from "./runtime-api-base";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const apiBaseUrl = getEffectiveApiBaseUrl();

  let response: Response;
  try {
    response = await fetch(`${apiBaseUrl}${path}`, {
      headers: {
        "Content-Type": "application/json",
        ...(init?.headers ?? {}),
      },
      ...init,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown network error";
    throw new Error(`Network error for ${apiBaseUrl}: ${message}`);
  }

  if (!response.ok) {
    throw new Error(`API ${response.status}: ${await response.text()}`);
  }

  return response.json() as Promise<T>;
}

export const api = {
  listAgents: async () => request<{ agents: AgentSummary[] }>("/agents"),
  getAgent: async (id: string) => request<AgentDetail>(`/agents/${id}`),
  getAgentStatus: async (id: string) => request<AgentStatus>(`/agents/${id}/status`),
  getAgentSkills: async (id: string) =>
    request<{ agent_id: string; skills: AgentSkill[] }>(`/agents/${id}/skills`),
  getCompany: async () => request<CompanyInfo>("/org/company"),
  getOrgTree: async () => request<{ root: OrgNode }>("/org/tree"),
  listOrgAgents: async () => request<{ agents: OrgAgent[] }>("/org/agents"),
  getSystemStatus: async () => request<SystemStatus>("/system/status"),
  getSystemHealth: async () => request<HealthStatus>("/system/health"),
  getPlugins: async () => request<PluginListResponse>("/system/plugins"),
  getPlugin: async (id: string) => request<PluginInfo>(`/system/plugins/${id}`),
  enablePlugin: async (id: string) =>
    request<PluginMutationResponse>(`/system/plugins/${id}/enable`, { method: "POST" }),
  disablePlugin: async (id: string) =>
    request<PluginMutationResponse>(`/system/plugins/${id}/disable`, { method: "POST" }),
  reloadPlugin: async (id: string) =>
    request<PluginMutationResponse>(`/system/plugins/${id}/reload`, { method: "POST" }),
  installPlugin: async (sourcePath: string) =>
    request<PluginMutationResponse>("/system/plugins/install", {
      method: "POST",
      body: JSON.stringify({ source_path: sourcePath }),
    }),
  uninstallPlugin: async (id: string) =>
    request<PluginMutationResponse>(`/system/plugins/${id}`, { method: "DELETE" }),
  trustPluginKey: async (alias: string, publicKeyPath: string) =>
    request<PluginMutationResponse>("/system/plugins/trust", {
      method: "POST",
      body: JSON.stringify({ alias, public_key_path: publicKeyPath }),
    }),
  signPlugin: async (id: string, privateKeyPath: string) =>
    request<PluginMutationResponse>(`/system/plugins/${id}/sign`, {
      method: "POST",
      body: JSON.stringify({ private_key_path: privateKeyPath }),
    }),
  getPluginLogs: async (id: string) => request<PluginLogsResponse>(`/system/plugins/${id}/logs`),
  getPluginDoctor: async () => request<PluginDoctorResponse>("/system/plugins/doctor"),
  listConversations: async () =>
    request<{ conversations: ConversationSummary[] }>("/messages/conversations"),
  createConversation: async (payload: {
    kind: ConversationKind;
    participant_ids: string[];
    participant_names: string[];
  }) =>
    request<Conversation>("/messages/conversations", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  getConversation: async (id: string) => request<Conversation>(`/messages/conversations/${id}`),
  listMessages: async (id: string) =>
    request<{ messages: Message[] }>(`/messages/conversations/${id}/messages`),
  sendMessage: async (payload: {
    conversation_id: string;
    sender_id: string;
    sender_name: string;
    recipient_id: string;
    recipient_name: string;
    content: string;
    message_type: string;
    reply_to_id?: string | null;
  }) =>
    request<Message>("/messages", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
};
