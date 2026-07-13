import { useQuery } from "@tanstack/react-query";
import type { Components } from "react-markdown";
import * as React from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";

import { ForumDeliveryImage } from "@/components/content/forum-delivery-image";
import { LightboxableImage } from "@/components/ui/image-lightbox";
import { api } from "@/lib/api/endpoints";
import { cn } from "@/lib/utils";
import type { ForumAttachment } from "@/lib/api/types";
import { mediaDeliveryRefetchInterval } from "@/lib/media-delivery";

export type ContentFormat = "plain_v1" | "markdown_v1";

/** Tracks whether markdown image renderers sit inside an <a> (linked image). */
const MarkdownInsideLinkContext = React.createContext(false);

function safeMarkdownUrl(url: string, key: string) {
  if (key === "src") {
    return /^\/__yourtj_asset__\/[1-9][0-9]*$/.test(url) ? url : "";
  }
  if (url.startsWith("/") && !url.startsWith("//")) return url;
  if (url.startsWith("#")) return url;
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:" ? parsed.toString() : "";
  } catch {
    return "";
  }
}

interface MarkdownNode {
  type?: string;
  url?: string;
  children?: MarkdownNode[];
}

function remarkPlatformImageReferences() {
  return (tree: MarkdownNode) => {
    const visit = (node: MarkdownNode) => {
      if (node.type === "image" && node.url) {
        const match = /^yourtj-asset:([1-9][0-9]*)$/.exec(node.url);
        node.url = match ? `/__yourtj_asset__/${match[1]}` : node.url;
      }
      node.children?.forEach(visit);
    };
    visit(tree);
  };
}

const baseComponents: Components = {
  a({ href, children, title }) {
    const isExternal = href?.startsWith("http://") || href?.startsWith("https://");
    return (
      <a
        href={href}
        title={title}
        target={isExternal ? "_blank" : undefined}
        rel={isExternal ? "noopener noreferrer nofollow ugc" : undefined}
        className="font-medium text-primary underline decoration-primary/35 underline-offset-4 hover:decoration-primary"
      >
        <MarkdownInsideLinkContext.Provider value={true}>
          {children}
        </MarkdownInsideLinkContext.Provider>
      </a>
    );
  },
  h1: ({ children }) => <h2 className="mb-3 mt-6 text-xl font-bold first:mt-0">{children}</h2>,
  h2: ({ children }) => <h3 className="mb-2 mt-5 text-lg font-semibold first:mt-0">{children}</h3>,
  h3: ({ children }) => <h4 className="mb-2 mt-4 font-semibold first:mt-0">{children}</h4>,
  p: ({ children }) => <p className="my-3 leading-7 first:mt-0 last:mb-0">{children}</p>,
  ul: ({ children }) => <ul className="my-3 list-disc space-y-1 pl-6">{children}</ul>,
  ol: ({ children }) => <ol className="my-3 list-decimal space-y-1 pl-6">{children}</ol>,
  blockquote: ({ children }) => (
    <blockquote className="my-4 border-l-4 border-primary/35 bg-muted/45 px-4 py-1 text-muted-foreground">
      {children}
    </blockquote>
  ),
  pre: ({ children }) => <pre className="my-4 overflow-x-auto rounded-lg bg-muted p-4 text-sm leading-6">{children}</pre>,
  code: ({ children, className }) => className ? (
    <code className={cn("font-mono", className)}>{children}</code>
  ) : (
    <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[0.9em]">{children}</code>
  ),
  table: ({ children }) => (
    <div className="my-4 overflow-x-auto">
      <table className="w-full border-collapse text-sm">{children}</table>
    </div>
  ),
  th: ({ children }) => <th className="border bg-muted px-3 py-2 text-left font-semibold">{children}</th>,
  td: ({ children }) => <td className="border px-3 py-2 align-top">{children}</td>,
};

function MarkdownForumImage(
  props: React.ComponentProps<typeof ForumDeliveryImage>,
) {
  const insideLink = React.useContext(MarkdownInsideLinkContext);
  return <ForumDeliveryImage {...props} enableLightbox={!insideLink} />;
}

function MarkdownPreviewImage({
  src,
  alt,
  width,
  height,
  className,
  onError,
}: {
  src: string;
  alt?: string;
  width?: number | null;
  height?: number | null;
  className?: string;
  onError?: React.ReactEventHandler<HTMLImageElement>;
}) {
  const insideLink = React.useContext(MarkdownInsideLinkContext);
  if (insideLink) {
    return (
      <img
        src={src}
        alt={alt}
        width={width ?? undefined}
        height={height ?? undefined}
        referrerPolicy="no-referrer"
        className={className}
        onError={onError}
      />
    );
  }
  return (
    <LightboxableImage
      src={src}
      alt={alt}
      width={width}
      height={height}
      referrerPolicy="no-referrer"
      className={className}
      onError={onError}
    />
  );
}

function OwnerMediaPreview({ assetId, alt }: { assetId: string; alt?: string }) {
  const upload = useQuery({
    queryKey: ["markdown-owner-media", "status", assetId],
    queryFn: () => api.myMediaUpload(assetId),
    refetchInterval: (query) => {
      const state = query.state.data;
      return state?.status === "pending" || state?.deliveryState === "processing" ? 3_000 : false;
    },
  });
  const pending = useQuery({
    queryKey: ["markdown-owner-media", "pending-preview", assetId],
    queryFn: () => api.myMediaPreview(assetId),
    enabled: upload.data?.status === "pending",
    staleTime: 60_000,
  });
  const delivery = useQuery({
    queryKey: ["markdown-owner-media", "delivery", assetId],
    queryFn: () => api.mediaUrl(assetId),
    enabled: upload.data?.status === "clean" && upload.data.deliveryState === "published",
    refetchInterval: (query) => mediaDeliveryRefetchInterval(query.state.data),
  });
  const [pendingUrl, setPendingUrl] = React.useState<string | null>(null);
  const retriedDeliveryUrl = React.useRef<string | null>(null);

  React.useEffect(() => {
    if (!pending.data) {
      setPendingUrl(null);
      return;
    }
    const nextUrl = URL.createObjectURL(pending.data);
    setPendingUrl(nextUrl);
    return () => URL.revokeObjectURL(nextUrl);
  }, [pending.data]);

  const label = alt?.trim() || "平台图片";
  if (upload.isLoading || pending.isLoading || delivery.isLoading) {
    return <span role="status" className="text-sm text-muted-foreground">正在加载图片预览</span>;
  }
  if (upload.data?.status === "pending") {
    return pendingUrl ? (
      <MarkdownPreviewImage
        src={pendingUrl}
        alt={`${label}（待审核预览）`}
        referrerPolicy="no-referrer"
        className="my-4 max-h-[36rem] max-w-full rounded-xl border object-contain opacity-80"
      />
    ) : (
      <span role="img" aria-label={`${label}待审核`} className="text-sm text-muted-foreground">
        待审核图片预览暂不可用
      </span>
    );
  }
  if (upload.data?.status === "clean" && upload.data.deliveryState === "processing") {
    return <span role="status" className="text-sm text-muted-foreground">图片正在生成安全版本</span>;
  }
  if (delivery.data?.url) {
    return (
      <MarkdownPreviewImage
        src={delivery.data.url}
        alt={label}
        width={delivery.data.width}
        height={delivery.data.height}
        referrerPolicy="no-referrer"
        className="my-4 max-h-[36rem] max-w-full rounded-xl border object-contain"
        onError={() => {
          if (retriedDeliveryUrl.current === delivery.data?.url) return;
          retriedDeliveryUrl.current = delivery.data?.url ?? null;
          void delivery.refetch();
        }}
      />
    );
  }
  return (
    <span role="img" aria-label={`图片不可用：${label}`} className="text-sm text-muted-foreground">
      图片当前不可用：{label}
    </span>
  );
}

function markdownComponents(
  attachments: ForumAttachment[],
  ownerPreviewAssetIds: readonly string[],
  onAttachmentDeliveryRefresh?: () => void,
): Components {
  const byReference = new Map(attachments.map((attachment) => [attachment.reference, attachment]));
  const ownerPreviewIds = new Set(ownerPreviewAssetIds);
  return {
    ...baseComponents,
    img({ alt, src }) {
      const match = /^\/__yourtj_asset__\/([1-9][0-9]*)$/.exec(src ?? "");
      const attachment = match ? byReference.get(`yourtj-asset:${match[1]}`) : undefined;
      if (!attachment) {
        if (match && ownerPreviewIds.has(match[1])) {
          return <OwnerMediaPreview assetId={match[1]} alt={alt ?? undefined} />;
        }
        return (
          <span
            role="img"
            aria-label={alt ? `图片不可用：${alt}` : "图片不可用"}
            className="inline-flex rounded border border-dashed px-2 py-1 text-sm text-muted-foreground"
          >
            图片当前不可用{alt ? `：${alt}` : ""}
          </span>
        );
      }
      return (
        <MarkdownForumImage
          attachment={attachment}
          onDeliveryRefresh={onAttachmentDeliveryRefresh}
          loading="lazy"
          decoding="async"
          className="my-4 max-h-[36rem] max-w-full rounded-xl border object-contain"
        />
      );
    },
  };
}

export function MarkdownContent({
  content,
  format,
  className,
  attachments = [],
  ownerPreviewAssetIds = [],
  onAttachmentDeliveryRefresh,
}: {
  content?: string | null;
  format: ContentFormat;
  className?: string;
  attachments?: ForumAttachment[];
  ownerPreviewAssetIds?: readonly string[];
  onAttachmentDeliveryRefresh?: () => void;
}) {
  if (!content) return null;
  if (format === "plain_v1") {
    return <div className={cn("whitespace-pre-wrap break-words leading-7", className)}>{content}</div>;
  }
  return (
    <div className={cn("min-w-0 break-words", className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkPlatformImageReferences]}
        rehypePlugins={[rehypeSanitize]}
        skipHtml
        urlTransform={safeMarkdownUrl}
        components={markdownComponents(
          attachments,
          ownerPreviewAssetIds,
          onAttachmentDeliveryRefresh,
        )}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
