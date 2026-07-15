import { ChevronLeft, ChevronRight, Download, Minus, Plus, X } from "lucide-react";
import * as React from "react";

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const ZOOM_LEVELS = [1, 1.5, 2, 3] as const;

export type LightboxImage = {
  src: string;
  alt?: string;
  width?: number | null;
  height?: number | null;
};

type ImageLightboxProps = {
  trigger: React.ReactElement;
  images: LightboxImage[];
  openIndex: number | null;
  onOpenChange: (open: boolean) => void;
  onIndexChange?: (index: number) => void;
};

export function ImageLightbox({
  trigger,
  images,
  openIndex,
  onOpenChange,
  onIndexChange,
}: ImageLightboxProps) {
  const open = openIndex !== null && images.length > 0;
  const safeIndex = openIndex === null
    ? 0
    : Math.min(Math.max(openIndex, 0), Math.max(images.length - 1, 0));
  const current = images[safeIndex];
  const canNavigate = images.length > 1;
  const [zoomIndex, setZoomIndex] = React.useState(0);
  const zoom = ZOOM_LEVELS[zoomIndex];

  React.useEffect(() => {
    setZoomIndex(0);
  }, [open, safeIndex]);

  const zoomIn = React.useCallback(() => {
    setZoomIndex((currentIndex) => Math.min(currentIndex + 1, ZOOM_LEVELS.length - 1));
  }, []);

  const zoomOut = React.useCallback(() => {
    setZoomIndex((currentIndex) => Math.max(currentIndex - 1, 0));
  }, []);

  const showPrevious = React.useCallback(() => {
    if (!canNavigate) return;
    const nextIndex = (safeIndex - 1 + images.length) % images.length;
    onIndexChange?.(nextIndex);
  }, [canNavigate, images.length, onIndexChange, safeIndex]);

  const showNext = React.useCallback(() => {
    if (!canNavigate) return;
    const nextIndex = (safeIndex + 1) % images.length;
    onIndexChange?.(nextIndex);
  }, [canNavigate, images.length, onIndexChange, safeIndex]);

  React.useEffect(() => {
    if (!open) return undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        showPrevious();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        showNext();
      } else if (event.key === "+" || event.key === "=") {
        event.preventDefault();
        zoomIn();
      } else if (event.key === "-") {
        event.preventDefault();
        zoomOut();
      } else if (event.key === "0") {
        event.preventDefault();
        setZoomIndex(0);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, showNext, showPrevious, zoomIn, zoomOut]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent
        className={cn(
          // Full-viewport transparent content so "empty" space around tall/narrow
          // images still receives clicks (Radix only closes on true outside clicks).
          "fixed inset-0 left-0 top-0 flex h-svh max-h-none w-screen max-w-none translate-x-0 translate-y-0 items-center justify-center gap-0 border-none bg-transparent p-0 shadow-none",
        )}
        aria-describedby={undefined}
        hideClose
        onClick={() => onOpenChange(false)}
      >
        <DialogTitle className="sr-only">
          {current?.alt?.trim() || "查看图片"}
          {canNavigate ? `（${safeIndex + 1}/${images.length}）` : ""}
        </DialogTitle>
        <div
          className="relative flex max-h-[min(96vh,960px)] max-w-[min(96vw,1200px)] items-center justify-center"
          onClick={(event) => event.stopPropagation()}
        >
          <div className="max-h-[min(92vh,920px)] max-w-[min(96vw,1200px)] overflow-auto rounded-lg">
            {current ? (
              <img
                src={current.src}
                alt={current.alt?.trim() || "查看图片"}
                width={current.width ?? undefined}
                height={current.height ?? undefined}
                referrerPolicy="no-referrer"
                className={cn(
                  "m-auto block rounded-lg object-contain shadow-2xl transition-[width,max-height] motion-reduce:transition-none",
                  zoom === 1 && "max-h-[min(92vh,920px)] max-w-full",
                )}
                style={zoom === 1 ? undefined : { width: `${zoom * 100}%`, maxWidth: "none" }}
              />
            ) : null}
          </div>
          <DialogClose
            className="absolute right-2 top-2 rounded-full bg-black/55 p-2 text-white transition-opacity hover:bg-black/75 focus-visible:ring-[3px] focus-visible:ring-ring/50"
            aria-label="关闭图片预览"
          >
            <X className="size-4" />
          </DialogClose>
          {canNavigate ? (
            <>
              <button
                type="button"
                className="absolute left-2 top-1/2 -translate-y-1/2 rounded-full bg-black/55 p-2 text-white transition-opacity hover:bg-black/75 focus-visible:ring-[3px] focus-visible:ring-ring/50"
                aria-label="上一张图片"
                onClick={showPrevious}
              >
                <ChevronLeft className="size-5" />
              </button>
              <button
                type="button"
                className="absolute right-2 top-1/2 -translate-y-1/2 rounded-full bg-black/55 p-2 text-white transition-opacity hover:bg-black/75 focus-visible:ring-[3px] focus-visible:ring-ring/50"
                aria-label="下一张图片"
                onClick={showNext}
              >
                <ChevronRight className="size-5" />
              </button>
            </>
          ) : null}
          {current ? (
            <div className="absolute bottom-2 left-1/2 flex -translate-x-1/2 items-center gap-1 rounded-full bg-black/65 p-1 text-white shadow-lg">
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 text-white hover:bg-white/15 hover:text-white"
                onClick={zoomOut}
                disabled={zoomIndex === 0}
                aria-label="缩小图片"
              >
                <Minus className="size-4" />
              </Button>
              <span className="min-w-20 text-center text-xs tabular-nums" aria-live="polite">
                {canNavigate ? `${safeIndex + 1}/${images.length} · ` : ""}{Math.round(zoom * 100)}%
              </span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 text-white hover:bg-white/15 hover:text-white"
                onClick={zoomIn}
                disabled={zoomIndex === ZOOM_LEVELS.length - 1}
                aria-label="放大图片"
              >
                <Plus className="size-4" />
              </Button>
              <Button
                asChild
                variant="ghost"
                size="icon"
                className="size-8 text-white hover:bg-white/15 hover:text-white"
              >
                <a
                  href={current.src}
                  download
                  target="_blank"
                  rel="noreferrer"
                  referrerPolicy="no-referrer"
                  aria-label="下载原图"
                >
                  <Download className="size-4" />
                </a>
              </Button>
            </div>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** Click-to-expand wrapper for a single delivery image. */
export function LightboxableImage({
  src,
  alt,
  width,
  height,
  className,
  images,
  imageIndex = 0,
  ...imageProps
}: Omit<React.ImgHTMLAttributes<HTMLImageElement>, "src" | "alt" | "width" | "height" | "onClick">
  & {
    src: string;
    alt?: string;
    width?: number | null;
    height?: number | null;
    images?: LightboxImage[];
    imageIndex?: number;
  }) {
  const [openIndex, setOpenIndex] = React.useState<number | null>(null);
  const gallery = images && images.length > 0
    ? images
    : [{ src, alt, width, height }];

  return (
    <ImageLightbox
      trigger={(
        <button
          type="button"
          className="group block max-w-full cursor-zoom-in border-0 bg-transparent p-0 text-left"
          aria-label={alt?.trim() ? `查看大图：${alt.trim()}` : "查看大图"}
        >
          <img
            {...imageProps}
            src={src}
            alt={alt}
            width={width ?? undefined}
            height={height ?? undefined}
            className={cn(className)}
          />
        </button>
      )}
      images={gallery}
      openIndex={openIndex}
      onOpenChange={(nextOpen) => setOpenIndex(nextOpen ? imageIndex : null)}
      onIndexChange={setOpenIndex}
    />
  );
}
