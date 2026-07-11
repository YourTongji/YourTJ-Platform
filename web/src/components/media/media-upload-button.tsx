import { ImagePlus, Loader2, Paperclip } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  type CompletedMediaUpload,
  type MediaKind,
  uploadMedia,
} from "@/lib/media-upload";

export function MediaUploadButton({
  kind,
  onUploaded,
  disabled,
  label,
}: {
  kind: MediaKind;
  onUploaded: (upload: CompletedMediaUpload) => void;
  disabled?: boolean;
  label?: string;
}) {
  const inputRef = React.useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = React.useState(false);

  async function handleFile(file: File) {
    setIsUploading(true);
    try {
      const upload = await uploadMedia(file, kind);
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
        accept={kind === "image" ? "image/jpeg,image/png,image/gif,image/webp" : "application/pdf"}
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
        onClick={() => inputRef.current?.click()}
        disabled={disabled || isUploading}
      >
        {isUploading ? <Loader2 className="size-4 animate-spin" /> : kind === "image" ? <ImagePlus className="size-4" /> : <Paperclip className="size-4" />}
        {isUploading ? "正在上传" : label ?? (kind === "image" ? "上传图片" : "上传文件")}
      </Button>
    </>
  );
}
