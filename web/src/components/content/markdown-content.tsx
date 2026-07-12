import type { Components } from "react-markdown";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";

import { cn } from "@/lib/utils";
import type { ForumAttachment } from "@/lib/api/types";

export type ContentFormat = "plain_v1" | "markdown_v1";

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
        {children}
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

function markdownComponents(attachments: ForumAttachment[]): Components {
  const byReference = new Map(attachments.map((attachment) => [attachment.reference, attachment]));
  return {
    ...baseComponents,
    img({ alt, src }) {
      const match = /^\/__yourtj_asset__\/([1-9][0-9]*)$/.exec(src ?? "");
      const attachment = match ? byReference.get(`yourtj-asset:${match[1]}`) : undefined;
      if (!attachment) {
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
        <img
          src={attachment.url}
          alt={attachment.alt}
          width={attachment.width ?? undefined}
          height={attachment.height ?? undefined}
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
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
}: {
  content?: string | null;
  format: ContentFormat;
  className?: string;
  attachments?: ForumAttachment[];
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
        components={markdownComponents(attachments)}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
