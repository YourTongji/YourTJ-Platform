import { api } from "@/lib/api/endpoints";
import type { MediaUsage } from "@/lib/api/types";

export type MediaKind = "image" | "file";

export interface CompletedMediaUpload {
  uploadId: string;
  ossKey: string;
  status: "pending";
}

const MAX_UPLOAD_BYTES = 20 * 1024 * 1024;
const IMAGE_TYPES = new Set(["image/jpeg", "image/png", "image/gif", "image/webp"]);
const FILE_TYPES = new Set(["application/pdf"]);

export function validateMediaFile(file: File, kind: MediaKind) {
  if (file.size <= 0 || file.size > MAX_UPLOAD_BYTES) {
    throw new Error("文件大小必须在 1 B 到 20 MB 之间");
  }
  const allowedTypes = kind === "image" ? IMAGE_TYPES : FILE_TYPES;
  if (!allowedTypes.has(file.type.toLowerCase())) {
    throw new Error(kind === "image" ? "仅支持 JPEG、PNG、GIF 或 WebP 图片" : "当前仅支持 PDF 文件");
  }
}

export function parseUploadCallbackData(value: unknown) {
  let parsed = value;
  if (typeof parsed === "string") {
    try {
      parsed = JSON.parse(parsed) as unknown;
    } catch {
      throw new Error("OSS 回调返回了无效数据");
    }
  }
  if (!parsed || typeof parsed !== "object") {
    throw new Error("OSS 回调没有返回上传记录");
  }
  const uploadId = (parsed as { uploadId?: unknown }).uploadId;
  if (typeof uploadId !== "string" && typeof uploadId !== "number") {
    throw new Error("OSS 回调没有返回上传记录");
  }
  return String(uploadId);
}

async function sha256Hex(file: File) {
  const digest = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/// Upload directly to the exact account-bound OSS object authorized by the backend.
export async function uploadMedia(
  file: File,
  kind: MediaKind,
  usage?: MediaUsage,
): Promise<CompletedMediaUpload> {
  validateMediaFile(file, kind);
  const [credentials, sha256] = await Promise.all([
    api.mediaUploadCredentials(kind, file.type.toLowerCase(), usage),
    sha256Hex(file),
  ]);
  if (credentials.expiration * 1_000 <= Date.now()) {
    throw new Error("上传凭证已过期，请重试");
  }

  const OSS = (await import("ali-oss")).default;
  const client = new OSS({
    region: credentials.region,
    authorizationV4: true,
    accessKeyId: credentials.accessKeyId,
    accessKeySecret: credentials.accessKeySecret,
    stsToken: credentials.securityToken,
    bucket: credentials.bucket,
    secure: true,
    timeout: 60_000,
  });
  const result = await client.put(credentials.ossKey, file, {
    mime: file.type,
    headers: { "Content-Type": file.type },
    callback: {
      url: credentials.callbackUrl,
      body: credentials.callbackBody,
      contentType: "application/json",
      customValue: { sha256 },
    },
  });

  return {
    uploadId: parseUploadCallbackData(result.data),
    ossKey: credentials.ossKey,
    status: "pending",
  };
}
