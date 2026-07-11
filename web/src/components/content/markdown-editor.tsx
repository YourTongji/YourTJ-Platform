import { markdown } from "@codemirror/lang-markdown";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { Bold, Code2, Italic, Link, List, ListOrdered, Quote } from "lucide-react";
import * as React from "react";

import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { MarkdownContent } from "@/components/content/markdown-content";
import { ForumImageAttachments } from "@/components/content/forum-image-attachments";
import type { MediaUsage } from "@/lib/api/types";

interface MarkdownAction {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  apply: (selected: string) => { value: string; selectionOffset: number };
}

const inlineActions: MarkdownAction[] = [
  {
    label: "加粗",
    icon: Bold,
    apply: (selected) => ({ value: `**${selected || "加粗文字"}**`, selectionOffset: 2 }),
  },
  {
    label: "斜体",
    icon: Italic,
    apply: (selected) => ({ value: `*${selected || "斜体文字"}*`, selectionOffset: 1 }),
  },
  {
    label: "行内代码",
    icon: Code2,
    apply: (selected) => ({ value: `\`${selected || "code"}\``, selectionOffset: 1 }),
  },
  {
    label: "链接",
    icon: Link,
    apply: (selected) => ({ value: `[${selected || "链接文字"}](https://)`, selectionOffset: 1 }),
  },
];

function referencedAssetIds(source: string) {
  const matches = source.matchAll(/!\[[^\]\n]*\]\(yourtj-asset:([1-9][0-9]*)\)/g);
  return Array.from(new Set(Array.from(matches, (match) => match[1])));
}

export function MarkdownEditor({
  value,
  onChange,
  label,
  maxLength,
  minHeight = 220,
  placeholder = "使用 Markdown 编写内容",
  attachmentUsage,
  attachmentAssetIds = [],
  onAttachmentAssetIdsChange,
  maxImages,
  onAttachmentsReadyChange,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  maxLength: number;
  minHeight?: number;
  placeholder?: string;
  attachmentUsage?: Extract<MediaUsage, "forum_thread" | "forum_comment">;
  attachmentAssetIds?: string[];
  onAttachmentAssetIdsChange?: (assetIds: string[]) => void;
  maxImages?: number;
  onAttachmentsReadyChange?: (isReady: boolean) => void;
}) {
  const editorRef = React.useRef<ReactCodeMirrorRef>(null);
  const [mode, setMode] = React.useState<"edit" | "preview">("edit");
  const extensions = React.useMemo(() => [
    markdown(),
    EditorView.lineWrapping,
    EditorState.changeFilter.of((transaction) => transaction.newDoc.length <= maxLength),
    EditorView.contentAttributes.of({ "aria-label": label }),
    EditorView.theme({
      "&": { backgroundColor: "transparent", color: "inherit", fontSize: "0.875rem" },
      ".cm-content": { minHeight: `${minHeight}px`, padding: "0.75rem", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" },
      ".cm-scroller": { overflow: "auto" },
      ".cm-gutters": { display: "none" },
      "&.cm-focused": { outline: "none" },
      ".cm-placeholder": { color: "hsl(var(--muted-foreground))" },
      ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { backgroundColor: "hsl(var(--primary) / 0.18)" },
    }),
  ], [label, maxLength, minHeight]);

  function replaceSelection(action: MarkdownAction) {
    const view = editorRef.current?.view;
    if (!view) return;
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to);
    const replacement = action.apply(selected);
    const selectedLength = selected ? selected.length : replacement.value.length - replacement.selectionOffset * 2;
    view.dispatch({
      changes: { from, to, insert: replacement.value },
      selection: {
        anchor: from + replacement.selectionOffset,
        head: from + replacement.selectionOffset + selectedLength,
      },
      scrollIntoView: true,
    });
    view.focus();
  }

  function prefixLines(prefix: string, fallback: string) {
    const view = editorRef.current?.view;
    if (!view) return;
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to) || fallback;
    const replacement = selected.split("\n").map((line) => `${prefix}${line}`).join("\n");
    view.dispatch({ changes: { from, to, insert: replacement }, scrollIntoView: true });
    view.focus();
  }

  function insertImage(assetId: string, alt: string) {
    if (attachmentAssetIds.includes(assetId)) return;
    const view = editorRef.current?.view;
    const safeAlt = alt
      .replaceAll("[", "")
      .replaceAll("]", "")
      .replaceAll("\\", "")
      .split(/\s+/)
      .filter(Boolean)
      .join(" ")
      .slice(0, 300) || "论坛图片";
    const reference = `![${safeAlt}](yourtj-asset:${assetId})`;
    if (view) {
      const { from, to } = view.state.selection.main;
      const before = from > 0 && !view.state.sliceDoc(from - 1, from).includes("\n") ? "\n\n" : "";
      const after = to < view.state.doc.length ? "\n\n" : "\n";
      view.dispatch({
        changes: { from, to, insert: `${before}${reference}${after}` },
        selection: { anchor: from + before.length + reference.length + after.length },
        scrollIntoView: true,
      });
      view.focus();
    } else {
      onChange(`${value}${value.endsWith("\n") || !value ? "" : "\n\n"}${reference}\n`);
    }
    onAttachmentAssetIdsChange?.([...attachmentAssetIds, assetId]);
  }

  function removeImage(assetId: string) {
    const escapedId = assetId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const pattern = new RegExp(
      `!?\\[[^\\]\\n]*\\]\\(yourtj-asset:${escapedId}\\)[\\t ]*\\n?`,
      "g",
    );
    const nextValue = value.replace(pattern, "").replace(/\n{3,}/g, "\n\n");
    const view = editorRef.current?.view;
    if (view) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: nextValue } });
    } else {
      onChange(nextValue);
    }
    onAttachmentAssetIdsChange?.(attachmentAssetIds.filter((currentId) => currentId !== assetId));
  }

  return (
    <div className="overflow-hidden rounded-lg border bg-background focus-within:ring-[3px] focus-within:ring-ring/35">
      <Tabs value={mode} onValueChange={(next) => setMode(next as "edit" | "preview")} className="gap-0">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-muted/30 px-2 py-1.5">
          <div className="flex items-center gap-0.5" role="toolbar" aria-label="Markdown 格式工具">
            {inlineActions.map((action) => (
              <Button
                key={action.label}
                type="button"
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={() => replaceSelection(action)}
                disabled={mode !== "edit"}
                aria-label={action.label}
              >
                <action.icon className="size-4" />
              </Button>
            ))}
            <Button type="button" variant="ghost" size="icon" className="size-8" onClick={() => prefixLines("> ", "引用内容")} disabled={mode !== "edit"} aria-label="引用">
              <Quote className="size-4" />
            </Button>
            <Button type="button" variant="ghost" size="icon" className="size-8" onClick={() => prefixLines("- ", "列表项")} disabled={mode !== "edit"} aria-label="无序列表">
              <List className="size-4" />
            </Button>
            <Button type="button" variant="ghost" size="icon" className="size-8" onClick={() => prefixLines("1. ", "列表项")} disabled={mode !== "edit"} aria-label="有序列表">
              <ListOrdered className="size-4" />
            </Button>
          </div>
          <TabsList className="h-8">
            <TabsTrigger value="edit" className="h-6 text-xs">编辑</TabsTrigger>
            <TabsTrigger value="preview" className="h-6 text-xs">预览</TabsTrigger>
          </TabsList>
        </div>
        <TabsContent value="edit">
          <CodeMirror
            ref={editorRef}
            value={value}
            onChange={(nextValue) => {
              onChange(nextValue);
              onAttachmentAssetIdsChange?.(referencedAssetIds(nextValue));
            }}
            extensions={extensions}
            placeholder={placeholder}
            basicSetup={{
              lineNumbers: false,
              foldGutter: false,
              highlightActiveLine: false,
              highlightActiveLineGutter: false,
              autocompletion: false,
            }}
          />
        </TabsContent>
        <TabsContent value="preview">
          <div className="p-4" style={{ minHeight }}>
            {value.trim() ? (
              <MarkdownContent content={value} format="markdown_v1" />
            ) : (
              <p className="text-sm text-muted-foreground">没有可预览的内容</p>
            )}
          </div>
        </TabsContent>
      </Tabs>
      <div className="flex items-center justify-between gap-3 border-t bg-muted/20 px-3 py-1.5 text-xs text-muted-foreground">
        <span>支持 CommonMark 与 GFM；不解析 HTML 或远程图片</span>
        <span className="shrink-0 tabular-nums">{value.length}/{maxLength}</span>
      </div>
      {attachmentUsage && onAttachmentAssetIdsChange && maxImages ? (
        <div className="border-t p-3">
          <ForumImageAttachments
            usage={attachmentUsage}
            assetIds={attachmentAssetIds}
            maxImages={maxImages}
            disabled={mode !== "edit"}
            onUpload={insertImage}
            onRemove={removeImage}
            onReadyChange={onAttachmentsReadyChange}
          />
        </div>
      ) : null}
    </div>
  );
}
