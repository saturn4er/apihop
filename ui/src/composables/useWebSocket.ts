import { ref, computed, onUnmounted } from "vue";
import {
  wsConnect,
  wsSend,
  wsDisconnect,
  wsSubscribe,
  type WsMessage,
  type WsStatus,
  type AuthConfig,
} from "@/api/client";

export function useWebSocket() {
  const connectionId = ref<string | null>(null);
  const status = ref<WsStatus>("disconnected");
  const messages = ref<WsMessage[]>([]);
  const filterText = ref("");
  const connectError = ref<string | null>(null);
  const unresolvedVariables = ref<string[]>([]);
  const environmentId = ref<string | undefined>(undefined);

  let unsubscribe: (() => void) | null = null;

  const filteredMessages = computed(() => {
    if (!filterText.value) return messages.value;
    const lower = filterText.value.toLowerCase();
    return messages.value.filter((m) => m.payload.toLowerCase().includes(lower));
  });

  async function connect(url: string, headers: Record<string, string>, auth?: AuthConfig, envId?: string) {
    connectError.value = null;
    unresolvedVariables.value = [];
    status.value = "connecting";
    environmentId.value = envId;
    try {
      const result = await wsConnect({ url, headers, auth, environment_id: envId });
      connectionId.value = result.connection_id;
      unresolvedVariables.value = result.unresolved_variables;
      status.value = "connected";

      unsubscribe = wsSubscribe(result.connection_id, (msg: WsMessage) => {
        if (msg.payload === "__ws_disconnected__") {
          status.value = "disconnected";
          connectionId.value = null;
          if (unsubscribe) {
            unsubscribe();
            unsubscribe = null;
          }
          return;
        }
        messages.value.push(msg);
      });
    } catch (e: any) {
      status.value = "disconnected";
      connectError.value = e?.message || String(e);
    }
  }

  async function send(payload: string, isBinary: boolean) {
    if (!connectionId.value) return;
    try {
      const msg = await wsSend(connectionId.value, payload, isBinary, environmentId.value);
      messages.value.push(msg);
    } catch (e: any) {
      connectError.value = e?.message || String(e);
    }
  }

  async function disconnect() {
    if (!connectionId.value) return;
    try {
      await wsDisconnect(connectionId.value);
    } catch {
      // ignore
    }
    status.value = "disconnected";
    if (unsubscribe) {
      unsubscribe();
      unsubscribe = null;
    }
    connectionId.value = null;
  }

  function clearLog() {
    messages.value = [];
  }

  // Cleanup on component unmount
  onUnmounted(() => {
    if (unsubscribe) {
      unsubscribe();
      unsubscribe = null;
    }
    // Disconnect if still connected
    if (connectionId.value) {
      wsDisconnect(connectionId.value).catch(() => {});
      connectionId.value = null;
      status.value = "disconnected";
    }
  });

  return {
    connectionId,
    status,
    messages,
    filterText,
    connectError,
    unresolvedVariables,
    filteredMessages,
    connect,
    send,
    disconnect,
    clearLog,
  };
}
