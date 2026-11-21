import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError, Catalog, CatalogAssessment, ChatEvent, Conversation, HardwareInfo, InstallPlan,
  Attachment, GenEvent, GenRequest, GeneratedImage, ImageInstallEvent, ImageOverview, InstalledModel, Message, ModelInfo,
  PullEvent, Recommendation, RequestInterpretation, RunningModel, RuntimeStatus,
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
  sendMessage: (conversationId: number, content: string, attachments: Attachment[], onEvent: (e: ChatEvent) => void) =>
    call<Message>("send_message", { conversationId, content, attachments, onEvent: channel(onEvent) }),
  setConversationModel: (conversationId: number, model: string) =>
    call<void>("set_conversation_model", { conversationId, model }),
  modelInfo: (model: string) => call<ModelInfo>("model_info", { model }),
  readAttachment: (path: string) => call<Attachment>("read_attachment", { path }),
  attachmentExtensions: () => call<string[]>("attachment_extensions"),

  imageOverview: () => call<ImageOverview>("image_overview"),
  imageLicenseAccepted: (modelId: string) => call<boolean>("image_license_accepted", { modelId }),
  installImageModel: (modelId: string, acceptLicense: boolean, onEvent: (e: ImageInstallEvent) => void) =>
    call<void>("install_image_model", { modelId, acceptLicense, onEvent: channel(onEvent) }),
  cancelImageInstall: (modelId: string) => call<boolean>("cancel_image_install", { modelId }),
  deleteImageModel: (modelId: string) => call<void>("delete_image_model", { modelId }),
  generateImage: (request: GenRequest, onEvent: (e: GenEvent) => void) =>
    call<GeneratedImage>("generate_image", { request, onEvent: channel(onEvent) }),
  cancelImageGeneration: () => call<boolean>("cancel_image_generation"),
  listGeneratedImages: () => call<GeneratedImage[]>("list_generated_images"),
  imageData: (id: number) => call<string>("image_data", { id }),
  exportImage: (id: number, dest: string) => call<void>("export_image", { id, dest }),
  deleteGeneratedImage: (id: number) => call<void>("delete_generated_image", { id }),
  cancelChat: (conversationId: number) => call<boolean>("cancel_chat", { conversationId }),

  interpretRequest: (text: string) => call<RequestInterpretation>("interpret_request", { text }),
};
