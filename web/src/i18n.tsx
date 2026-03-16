import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export type LocaleCode =
  | "en"
  | "zh-CN";

type Messages = Record<string, string>;

const STORAGE_KEY = "clawlegion.locale";

export const LOCALE_OPTIONS: Array<{ code: LocaleCode; label: string }> = [
  { code: "en", label: "English" },
  { code: "zh-CN", label: "Simplified Chinese" },
];

const RTL_LOCALES = new Set<LocaleCode>([]);

const EN_MESSAGES: Messages = {
  "app.nav.dashboard": "Dashboard",
  "app.nav.agents": "Agents",
  "app.nav.org": "Org Chart",
  "app.nav.messages": "Message Monitor",
  "app.nav.system": "System",
  "app.description": "Frontend console for monitoring agent squads, budget posture, and private conversations.",
  "app.liveSummary": "Live Summary",
  "app.system": "System",
  "app.activeAgents": "Active Agents {active}/{total}",
  "app.budgetRemaining": "Budget Remaining {value}",
  "app.badge.hashRouter": "Hash Router",
  "app.badge.polling": "Polling IM",
  "app.badge.static": "Static Deploy Ready",
  "app.language": "Language",
  "common.na": "N/A",
  "common.notSelected": "Not Selected",
  "common.seconds": "s",
  "common.lastPolled": "Last polled {time}",

  "dashboard.systemStatus": "System Status",
  "dashboard.uptime": "Uptime {seconds}s",
  "dashboard.agentActive": "Active Agents",
  "dashboard.polling": "Polled every 5 seconds",
  "dashboard.budgetRemaining": "Budget Remaining",
  "dashboard.usage": "Used {percent}%",
  "dashboard.pluginsLoaded": "Plugins Loaded",
  "dashboard.memory": "Memory {value} MB",
  "dashboard.health.title": "Health Checks",
  "dashboard.health.subtitle": "Per-subsystem status view",
  "dashboard.snapshot.title": "Agent Snapshot",
  "dashboard.snapshot.subtitle": "Recently registered agents and states",

  "agents.title": "Agent Roster",
  "agents.subtitle": "Search, filter, and inspect costs and capabilities",
  "agents.search": "Search by agent name, role, or title",
  "agents.table.agent": "Agent",
  "agents.table.status": "Status",
  "agents.table.heartbeat": "Heartbeat",
  "agents.table.budget": "Budget",

  "org.title": "Organization Map",
  "org.subtitle": "Hierarchy and role positioning with React Flow",
  "org.budget.title": "Budget Summary",
  "org.budget.subtitle": "Always-on budget panel for org view",
  "org.monthlyBudget": "Monthly Budget",
  "org.spentBudget": "Spent",
  "org.topSpender": "Top Spender",
  "org.list.title": "Agent List",
  "org.list.subtitle": "Click a node and inspect the matching entry",
  "org.depth": "depth",

  "messages.list.title": "Conversations",
  "messages.list.subtitle": "Monitor any Agent-Agent or User-Agent conversation",
  "messages.create.agentAgent": "Create Agent-Agent Conversation",
  "messages.select.left": "Select left agent",
  "messages.select.right": "Select right agent",
  "messages.startMonitor": "Start Monitoring",
  "messages.create.userAgent": "Create User-Agent Conversation",
  "messages.select.agent": "Select agent",
  "messages.privateChat": "Private chat with agent",
  "messages.noMessages": "No messages yet",
  "messages.thread.title": "Message Thread",
  "messages.thread.subtitle": "2-second polling with text send support",
  "messages.input.placeholder": "Type a message for the current conversation target agent",
  "messages.send": "Send Message",
  "messages.participants.title": "Participants",
  "messages.participants.subtitle": "Current conversation context",
  "messages.currentConversation": "Current conversation: {kind}",
  "messages.lastUpdated": "Last updated: {time}",

  "system.title": "System Status",
  "system.subtitle": "Node status and runtime summary",
  "system.status": "Status",
  "system.version": "Version",
  "system.uptime": "Uptime",
  "system.memory": "Memory Usage",
  "system.health.title": "Health Checks",
  "system.health.subtitle": "Database, LLM provider, and plugin system status",

  "agent.loading": "Loading agent detail...",
  "agent.notFound": "Agent detail not found.",
  "agent.header": "Agent Detail",
  "agent.runtime.title": "Runtime",
  "agent.runtime.subtitle": "State, task, and heartbeat",
  "agent.status": "Status",
  "agent.currentTask": "Current Task",
  "agent.idle": "Idle",
  "agent.lastHeartbeat": "Last Heartbeat",
  "agent.cost.title": "Cost",
  "agent.cost.subtitle": "Budget and token usage",
  "agent.budgetRemaining": "Budget Remaining",
  "agent.tokenTotal": "Total Tokens",
  "agent.totalCost": "Total Cost",
  "agent.skills.title": "Capabilities & Skills",
  "agent.skills.subtitle": "Skills are fetched from API",
  "agent.capabilities": "Capabilities",
  "agent.skills": "Skills",
};

const TRANSLATIONS: Record<LocaleCode, Messages> = {
  en: EN_MESSAGES,
  "zh-CN": {
    ...EN_MESSAGES,
    "app.nav.dashboard": "态势总览",
    "app.nav.agents": "Agent",
    "app.nav.org": "组织图",
    "app.nav.messages": "消息监视",
    "app.nav.system": "系统状态",
    "app.description": "用于监控 Agent 编队、组织预算和私聊会话的前端工作台。",
    "app.liveSummary": "实时摘要",
    "app.system": "系统",
    "app.activeAgents": "活跃 Agent {active}/{total}",
    "app.budgetRemaining": "预算剩余 {value}",
    "app.language": "语言",
    "common.na": "暂无",
    "common.notSelected": "未选择",
    "common.lastPolled": "最近轮询 {time}",
    "dashboard.systemStatus": "系统状态",
    "dashboard.uptime": "运行 {seconds}s",
    "dashboard.agentActive": "Agent 活跃数",
    "dashboard.polling": "按 5 秒轮询",
    "dashboard.budgetRemaining": "预算剩余",
    "dashboard.usage": "已使用 {percent}%",
    "dashboard.pluginsLoaded": "插件加载",
    "dashboard.memory": "内存 {value} MB",
    "dashboard.health.title": "健康检查",
    "dashboard.health.subtitle": "关键子系统逐项展示",
    "dashboard.snapshot.title": "Agent 快照",
    "dashboard.snapshot.subtitle": "最近注册的 Agent 与状态",
    "agents.title": "Agent 编队",
    "agents.subtitle": "搜索、筛选并查看详细成本和能力",
    "agents.search": "搜索 Agent 名称、角色或头衔",
    "agents.table.status": "状态",
    "agents.table.heartbeat": "心跳",
    "agents.table.budget": "预算",
    "org.title": "组织战术图",
    "org.subtitle": "以 React Flow 展示层级关系和角色定位",
    "org.budget.title": "预算摘要",
    "org.budget.subtitle": "组织页常驻预算信息",
    "org.monthlyBudget": "月预算",
    "org.spentBudget": "已花费",
    "org.list.title": "Agent 列表",
    "org.list.subtitle": "点击节点后可在这里查看对应条目",
    "messages.list.title": "会话列表",
    "messages.list.subtitle": "监视任意 Agent-Agent 或 User-Agent 会话",
    "messages.create.agentAgent": "创建 Agent-Agent 会话",
    "messages.select.left": "选择左侧 Agent",
    "messages.select.right": "选择右侧 Agent",
    "messages.startMonitor": "开始监视",
    "messages.create.userAgent": "创建 User-Agent 会话",
    "messages.select.agent": "选择 Agent",
    "messages.privateChat": "与 Agent 私聊",
    "messages.noMessages": "暂无消息",
    "messages.thread.title": "消息线程",
    "messages.thread.subtitle": "两秒轮询，支持发送文本消息",
    "messages.input.placeholder": "输入发给当前会话目标 Agent 的文本消息",
    "messages.send": "发送消息",
    "messages.participants.title": "参与者信息",
    "messages.participants.subtitle": "当前会话上下文",
    "messages.currentConversation": "当前会话：{kind}",
    "messages.lastUpdated": "最近更新时间：{time}",
    "system.title": "系统状态",
    "system.subtitle": "节点级状态与运行时摘要",
    "system.status": "状态",
    "system.version": "版本",
    "system.uptime": "运行时长",
    "system.memory": "内存使用",
    "system.health.title": "健康检查",
    "system.health.subtitle": "数据库、LLM Provider、插件系统逐项状态",
    "agent.loading": "加载 Agent 详情中…",
    "agent.notFound": "未找到 Agent 详情。",
    "agent.runtime.title": "运行态",
    "agent.runtime.subtitle": "状态、任务与心跳",
    "agent.currentTask": "当前任务",
    "agent.idle": "空闲",
    "agent.lastHeartbeat": "最近心跳",
    "agent.cost.title": "成本态",
    "agent.cost.subtitle": "预算与 token 消耗",
    "agent.budgetRemaining": "预算剩余",
    "agent.tokenTotal": "Token 总量",
    "agent.totalCost": "累计花费",
    "agent.skills.title": "能力与技能",
    "agent.skills.subtitle": "技能单独从 API 拉取",
  },
};

function interpolate(template: string, vars?: Record<string, string | number>) {
  if (!vars) return template;
  return Object.entries(vars).reduce((result, [key, value]) => {
    return result.replaceAll(`{${key}}`, String(value));
  }, template);
}

function isLocaleCode(value: string): value is LocaleCode {
  return LOCALE_OPTIONS.some((item) => item.code === value);
}

function readStoredLocale(): LocaleCode | null {
  const storage = window.localStorage as Partial<Storage> | undefined;
  if (!storage || typeof storage.getItem !== "function") {
    return null;
  }
  const value = storage.getItem(STORAGE_KEY);
  if (value && isLocaleCode(value)) {
    return value;
  }
  return null;
}

function writeStoredLocale(locale: LocaleCode) {
  const storage = window.localStorage as Partial<Storage> | undefined;
  if (!storage || typeof storage.setItem !== "function") {
    return;
  }
  storage.setItem(STORAGE_KEY, locale);
}

type I18nContextValue = {
  locale: LocaleCode;
  intlLocale: string;
  setLocale: (locale: LocaleCode) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
  localeOptions: Array<{ code: LocaleCode; label: string }>;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<LocaleCode>(() => readStoredLocale() ?? "en");

  useEffect(() => {
    writeStoredLocale(locale);
    document.documentElement.lang = locale;
    document.documentElement.dir = RTL_LOCALES.has(locale) ? "rtl" : "ltr";
  }, [locale]);

  const value = useMemo<I18nContextValue>(() => {
    const t = (key: string, vars?: Record<string, string | number>) => {
      const message = TRANSLATIONS[locale][key] ?? EN_MESSAGES[key] ?? key;
      return interpolate(message, vars);
    };

    return {
      locale,
      intlLocale: locale,
      setLocale,
      t,
      localeOptions: LOCALE_OPTIONS,
    };
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return context;
}
