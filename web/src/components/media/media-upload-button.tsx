import { ImagePlus, Loader2, Paperclip } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  type CompletedMediaUpload,
  type MediaKind,
  uploadMedia,
} from "@/lib/media-upload";
import type { MediaUsage } from "@/lib/api/types";
import { STATIC_IMAGE_ACCEPT, STATIC_IMAGE_REUPLOAD_MESSAGE } from "@/lib/media-policy";

export function MediaUploadButton({
  kind,
  onUploaded,
  disabled,
  label,
  usage,
}: {
  kind: MediaKind;
  onUploaded: (upload: CompletedMediaUpload) => void;
  disabled?: boolean;
  label?: string;
  usage?: MediaUsage;
}) {
  const inputRef = React.useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = React.useState(false);

  async function handleFile(file: File) {
    setIsUploading(true);
    try {
      const upload = await uploadMedia(file, kind, usage);
      onUploaded(upload);
      toast.success("文件已上传，审核通过后才会公开显示");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "上传失败");
    } finally {
      setIsUploading(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  }

  return (
    <>
      <input
        ref={inputRef}
        type="file"
        className="sr-only"
        accept={kind === "image" ? STATIC_IMAGE_ACCEPT : "application/pdf"}
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file) void handleFile(file);
        }}
        tabIndex={-1}
        aria-hidden="true"
      />
      <Button
        type="button"
        variant="outline"
        title={kind === "image" ? STATIC_IMAGE_REUPLOAD_MESSAGE : undefined}
        onClick={() => inputRef.current?.click()}
        disabled={disabled || isUploading}
      >
        {isUploading ? <Loader2 className="size-4 motion-safe:animate-spin" aria-hidden="true" /> : kind === "image" ? <ImagePlus className="size-4" aria-hidden="true" /> : <Paperclip className="size-4" aria-hidden="true" />}
        {isUploading ? "正在上传" : label ?? (kind === "image" ? "上传图片" : "上传文件")}
      </Button>
    </>
  );
}
