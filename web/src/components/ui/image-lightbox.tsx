import { ChevronLeft, ChevronRight, X } from "lucide-react";
import * as React from "react";

import { Dialog, DialogClose, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

export type LightboxImage = {
  src: string;
  alt?: string;
  width?: number | null;
  height?: number | null;
};

type ImageLightboxProps = {
  images: LightboxImage[];
  openIndex: number | null;
  onOpenChange: (open: boolean) => void;
  onIndexChange?: (index: number) => void;
};

export function ImageLightbox({
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
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, showNext, showPrevious]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
          {current ? (
            <img
              src={current.src}
              alt={current.alt?.trim() || "查看图片"}
              width={current.width ?? undefined}
              height={current.height ?? undefined}
              referrerPolicy="no-referrer"
              className="max-h-[min(92vh,920px)] max-w-full rounded-lg object-contain shadow-2xl"
            />
          ) : null}
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
    <>
      <button
        type="button"
        className="group block max-w-full cursor-zoom-in border-0 bg-transparent p-0 text-left"
        onClick={() => setOpenIndex(imageIndex)}
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
      <ImageLightbox
        images={gallery}
        openIndex={openIndex}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setOpenIndex(null);
        }}
        onIndexChange={setOpenIndex}
      />
    </>
  );
}
