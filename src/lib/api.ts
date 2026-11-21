import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError, Catalog, CatalogAssessment, ChatEvent, Conversation, HardwareInfo, InstallPlan,
  InstalledModel, Message, PullEvent, Recommendation, RequestInterpretation, RunningModel, RuntimeStatus,
} from "./types";

/** Normalise anything thrown by `invoke` into an AppError. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "message" in e) {
    const o = e as { kind?: string; message: unknown };
    return { kind: o.kind ?? "unknown", message: String(o.message) };
  }
  return { kind: "unknown", message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

function channel<T>(onEvent: (e: T) => void): Channel<T> {
  const ch = new Channel<T>();
  ch.onmessage = onEvent;
  return ch;
}

export const api = {
  hardware: () => call<HardwareInfo>("get_hardware_info"),
  catalog: () => call<Catalog>("get_catalog"),
  assessCatalog: () => call<CatalogAssessment>("assess_catalog"),
  recommendations: (useCase?: string) =>
    call<Recommendation[]>("get_recommendations", { useCase: useCase ?? null }),

  runtimeStatus: () => call<RuntimeStatus>("runtime_status"),
  startRuntime: () => call<RuntimeStatus>("start_runtime"),
  listInstalled: () => call<InstalledModel[]>("list_installed"),
  listRunning: () => call<RunningModel[]>("list_running"),
  loadModel: (model: string) => call<void>("load_model", { model }),
  unloadModel: (model: string) => call<void>("unload_model", { model }),
  deleteModel: (model: string) => call<void>("delete_model", { model }),

  prepareInstall: (tag: string) => call<InstallPlan>("prepare_install", { tag }),
  installModel: (tag: string, acceptLicense: boolean, onEvent: (e: PullEvent) => void) =>
    call<void>("install_model", { tag, acceptLicense, onEvent: channel(onEvent) }),
  cancelInstall: (tag: string) => call<boolean>("cancel_install", { tag }),
  activeInstalls: () => call<string[]>("active_installs"),

  listConversations: () => call<Conversation[]>("list_conversations"),
  createConversation: (model: string) => call<Conversation>("create_conversation", { model }),
  getMessages: (conversationId: number) => call<Message[]>("get_messages", { conversationId }),
  deleteConversation: (conversationId: number) => call<void>("delete_conversation", { conversationId }),
  sendMessage: (conversationId: number, content: string, onEvent: (e: ChatEvent) => void) =>
    call<Message>("send_message", { conversationId, content, onEvent: channel(onEvent) }),
  cancelChat: (conversationId: number) => call<boolean>("cancel_chat", { conversationId }),

  interpretRequest: (text: string) => call<RequestInterpretation>("interpret_request", { text }),
};
