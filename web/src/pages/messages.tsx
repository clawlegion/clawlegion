import { useEffect, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";

import { useAgents, useConversation, useConversations, useMessages } from "../hooks/use-api";
import { api } from "../lib/api";
import { formatDateTime } from "../lib/utils";
import { queryClient } from "../query-client";
import { SectionCard } from "../components/section-card";
import { useI18n } from "../i18n";

const USER_PARTICIPANT = {
  id: "user-console",
  name: "Console User",
};

export function MessagesPage() {
  const agents = useAgents();
  const conversations = useConversations();
  const [selectedConversationId, setSelectedConversationId] = useState<string>("");
  const [leftAgentId, setLeftAgentId] = useState<string>("");
  const [rightAgentId, setRightAgentId] = useState<string>("");
  const [messageDraft, setMessageDraft] = useState("");
  const [chatTargetId, setChatTargetId] = useState<string>("");
  const activeConversation = useConversation(selectedConversationId);
  const messages = useMessages(selectedConversationId);
  const { t, intlLocale } = useI18n();

  useEffect(() => {
    if (!selectedConversationId && conversations.data?.conversations[0]?.conversation_id) {
      setSelectedConversationId(conversations.data.conversations[0].conversation_id);
    }
  }, [conversations.data?.conversations, selectedConversationId]);

  const createConversation = useMutation({
    mutationFn: api.createConversation,
    onSuccess: (conversation) => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      setSelectedConversationId(conversation.conversation_id);
    },
  });

  const sendMessage = useMutation({
    mutationFn: api.sendMessage,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", selectedConversationId] });
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      setMessageDraft("");
    },
  });

  const currentPeer = useMemo(() => {
    return activeConversation.data?.participants.find((participant) => participant.id !== USER_PARTICIPANT.id);
  }, [activeConversation.data?.participants]);

  return (
    <div className="grid gap-5 xl:grid-cols-[0.34fr_0.96fr_0.45fr]">
      <SectionCard title={t("messages.list.title")} subtitle={t("messages.list.subtitle")}>
        <div className="space-y-3">
          <div className="rounded-2xl border border-black/10 bg-stone-50 p-3">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">{t("messages.create.agentAgent")}</p>
            <div className="mt-3 grid gap-2">
              <select value={leftAgentId} onChange={(e) => setLeftAgentId(e.target.value)} className="rounded-xl border border-black/10 bg-white px-3 py-2 text-sm">
                <option value="">{t("messages.select.left")}</option>
                {agents.data?.agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
              </select>
              <select value={rightAgentId} onChange={(e) => setRightAgentId(e.target.value)} className="rounded-xl border border-black/10 bg-white px-3 py-2 text-sm">
                <option value="">{t("messages.select.right")}</option>
                {agents.data?.agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
              </select>
              <button
                className="rounded-xl bg-graphite px-3 py-2 text-sm text-sand disabled:opacity-50"
                disabled={!leftAgentId || !rightAgentId || leftAgentId === rightAgentId}
                onClick={() => {
                  const items = agents.data?.agents ?? [];
                  const left = items.find((item) => item.id === leftAgentId);
                  const right = items.find((item) => item.id === rightAgentId);
                  if (!left || !right) return;
                  createConversation.mutate({
                    kind: "agent-agent",
                    participant_ids: [left.id, right.id],
                    participant_names: [left.name, right.name],
                  });
                }}
              >
                {t("messages.startMonitor")}
              </button>
            </div>
          </div>
          <div className="rounded-2xl border border-black/10 bg-stone-50 p-3">
            <p className="text-xs uppercase tracking-[0.24em] text-steel">{t("messages.create.userAgent")}</p>
            <div className="mt-3 grid gap-2">
              <select value={chatTargetId} onChange={(e) => setChatTargetId(e.target.value)} className="rounded-xl border border-black/10 bg-white px-3 py-2 text-sm">
                <option value="">{t("messages.select.agent")}</option>
                {agents.data?.agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
              </select>
              <button
                className="rounded-xl bg-signal px-3 py-2 text-sm text-white disabled:opacity-50"
                disabled={!chatTargetId}
                onClick={() => {
                  const target = agents.data?.agents.find((item) => item.id === chatTargetId);
                  if (!target) return;
                  createConversation.mutate({
                    kind: "user-agent",
                    participant_ids: [USER_PARTICIPANT.id, target.id],
                    participant_names: [USER_PARTICIPANT.name, target.name],
                  });
                }}
              >
                {t("messages.privateChat")}
              </button>
            </div>
          </div>
          <div className="space-y-2">
            {conversations.data?.conversations.map((conversation) => (
              <button
                key={conversation.conversation_id}
                className={`w-full rounded-2xl border px-4 py-3 text-left ${
                  selectedConversationId === conversation.conversation_id
                    ? "border-signal bg-orange-50"
                    : "border-black/10 bg-white/70"
                }`}
                onClick={() => setSelectedConversationId(conversation.conversation_id)}
              >
                <div className="flex items-center justify-between gap-3">
                  <p className="font-medium">{conversation.participant_names.join(" <> ")}</p>
                  <span className="text-xs uppercase tracking-[0.2em] text-steel">{conversation.kind}</span>
                </div>
                <p className="mt-2 line-clamp-2 text-sm text-graphite/65">{conversation.last_message_preview ?? t("messages.noMessages")}</p>
              </button>
            ))}
          </div>
        </div>
      </SectionCard>

      <SectionCard title={t("messages.thread.title")} subtitle={t("messages.thread.subtitle")}>
        <div className="flex h-[760px] flex-col">
          <div className="flex-1 space-y-3 overflow-y-auto rounded-[24px] border border-black/10 bg-stone-50 p-4">
            {messages.data?.messages.map((message) => {
              const outgoing = message.sender_id === USER_PARTICIPANT.id;
              return (
                <div key={message.message_id} className={`flex ${outgoing ? "justify-end" : "justify-start"}`}>
                  <div className={`max-w-[72%] rounded-[24px] px-4 py-3 ${outgoing ? "bg-graphite text-sand" : "bg-white text-graphite"}`}>
                    <p className="text-xs uppercase tracking-[0.2em] opacity-65">{message.sender_name}</p>
                    <p className="mt-2 whitespace-pre-wrap text-sm leading-6">{message.content}</p>
                    <p className="mt-3 text-right text-xs opacity-60">{formatDateTime(message.timestamp, intlLocale)}</p>
                  </div>
                </div>
              );
            })}
          </div>
          <div className="mt-4 grid gap-3 rounded-[24px] border border-black/10 bg-white/80 p-4">
            <textarea
              value={messageDraft}
              onChange={(event) => setMessageDraft(event.target.value)}
              className="min-h-28 rounded-2xl border border-black/10 bg-stone-50 p-4 text-sm outline-none"
              placeholder={t("messages.input.placeholder")}
            />
            <div className="flex justify-end">
              <button
                className="rounded-xl bg-graphite px-4 py-2 text-sm text-sand disabled:opacity-50"
                disabled={!selectedConversationId || !messageDraft.trim()}
                onClick={() => {
                  const peer = currentPeer ?? activeConversation.data?.participants[0];
                  if (!peer) return;
                  sendMessage.mutate({
                    conversation_id: selectedConversationId,
                    sender_id: USER_PARTICIPANT.id,
                    sender_name: USER_PARTICIPANT.name,
                    recipient_id: peer.id,
                    recipient_name: peer.name,
                    content: messageDraft,
                    message_type: "text",
                    reply_to_id: null,
                  });
                }}
              >
                {t("messages.send")}
              </button>
            </div>
          </div>
        </div>
      </SectionCard>

      <SectionCard title={t("messages.participants.title")} subtitle={t("messages.participants.subtitle")}>
        <div className="space-y-3">
          {activeConversation.data?.participants.map((participant) => (
            <div key={participant.id} className="rounded-2xl border border-black/10 bg-stone-50 px-4 py-3">
              <p className="font-medium">{participant.name}</p>
              <p className="text-sm text-graphite/65">{participant.id}</p>
            </div>
          ))}
          <div className="rounded-2xl border border-black/10 bg-white/80 px-4 py-3 text-sm text-graphite/70">
            <p>{t("messages.currentConversation", { kind: activeConversation.data?.kind ?? t("common.notSelected") })}</p>
            <p className="mt-2">{t("messages.lastUpdated", { time: formatDateTime(activeConversation.data?.updated_at, intlLocale) })}</p>
          </div>
        </div>
      </SectionCard>
    </div>
  );
}
