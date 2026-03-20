export interface AgentSummary {
  id: string;
  name: string;
  role: string;
  title: string;
  status: string;
  icon?: string | null;
  reports_to?: string | null;
  last_heartbeat?: string | null;
}

export interface AgentDetail extends AgentSummary {
  capabilities: string[];
  skills: string[];
  tasks_completed: number;
  tasks_pending: number;
}

export interface AgentStatus {
  agent_id: string;
  status: string;
  current_task?: string | null;
  last_heartbeat?: string | null;
  heartbeat_interval_secs: number;
}

export interface AgentSkill {
  name: string;
  version: string;
  description: string;
  execution_count: number;
}

export interface CompanyInfo {
  company_id: string;
  company_name: string;
  issue_prefix: string;
  agent_count: number;
  created_at: string;
}

export interface OrgNode {
  node_id: string;
  name: string;
  role: string;
  title: string;
  icon?: string | null;
  depth: number;
  children: OrgNode[];
}

export interface OrgAgent {
  id: string;
  name: string;
  role: string;
  title: string;
  depth: number;
  parent_id?: string | null;
  direct_reports_count: number;
}

export interface HealthStatus {
  status: string;
  checks: {
    database: string;
    llm_provider: string;
    plugin_system: string;
  };
}

export interface PluginHealthSummary {
  healthy: number;
  degraded: number;
  failed: number;
}

export interface SystemStatus {
  status: string;
  uptime_secs: number;
  version: string;
  agents_total: number;
  agents_active: number;
  plugins_loaded: number;
  plugins_active: number;
  memory_usage_mb: number;
  plugin_health: PluginHealthSummary;
}

export interface PluginDependency {
  name: string;
  version_req: string;
  optional: boolean;
}

export interface PluginCapability {
  id: string;
  kind: string;
  display_name?: string | null;
  description?: string | null;
  interface?: string | null;
  tags: string[];
}

export interface PluginPermission {
  scope: string;
  resource?: string | null;
  reason?: string | null;
}

export interface PluginManifest {
  id: string;
  version: string;
  api_version: string;
  runtime: string;
  entrypoint: string;
  capabilities: PluginCapability[];
  permissions: PluginPermission[];
  dependencies: PluginDependency[];
  compatible_host_versions: string[];
}

export interface PluginInfo {
  id: string;
  plugin_type: string;
  state: string;
  enabled: boolean;
  load_path?: string | null;
  manifest_path?: string | null;
  health?: string | null;
  errors: string[];
  manifest: PluginManifest;
}

export interface PluginListResponse {
  plugins: PluginInfo[];
  capability_index: Record<string, string[]>;
  bridge_index: Record<string, string[]>;
  sentinel_triggers: Array<{
    id: string;
    agent_id: string;
    enabled: boolean;
    condition: string;
  }>;
}

export interface PluginMutationResponse {
  ok: boolean;
  plugin?: PluginInfo | null;
  detail: string;
}

export interface PluginRuntimeLog {
  plugin_id: string;
  runtime: string;
  message: string;
}

export interface PluginLogsResponse {
  plugin_id: string;
  logs: PluginRuntimeLog[];
  health?: string | null;
  last_error?: string | null;
}

export interface PluginDoctorResponse {
  reports: Array<Record<string, unknown>>;
}

export type ConversationKind = "agent-agent" | "user-agent";

export interface ConversationSummary {
  conversation_id: string;
  kind: ConversationKind;
  participant_ids: string[];
  participant_names: string[];
  last_message_preview?: string | null;
  last_message_at?: string | null;
  unread_count: number;
}

export interface ConversationParticipant {
  id: string;
  name: string;
}

export interface Conversation {
  conversation_id: string;
  kind: ConversationKind;
  participants: ConversationParticipant[];
  created_at: string;
  updated_at: string;
}

export interface Message {
  message_id: string;
  conversation_id: string;
  sender_id: string;
  sender_name: string;
  recipient_id: string;
  recipient_name: string;
  content: string;
  timestamp: string;
  message_type: string;
  reply_to_id?: string | null;
  read: boolean;
}
