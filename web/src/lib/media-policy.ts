export const STATIC_IMAGE_MIME_TYPES = ["image/jpeg", "image/png", "image/webp"] as const;

export const STATIC_IMAGE_ACCEPT = STATIC_IMAGE_MIME_TYPES.join(",");

export const STATIC_IMAGE_REUPLOAD_MESSAGE =
  "仅支持静态 JPEG、PNG 或 WebP 图片；GIF 或其他动图请转换为静态图片后重新上传";

const staticImageMimeTypes = new Set<string>(STATIC_IMAGE_MIME_TYPES);

export function isSupportedStaticImageContentType(contentType: string) {
  return staticImageMimeTypes.has(contentType.toLowerCase());
}
