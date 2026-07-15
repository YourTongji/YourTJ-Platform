export type ShareForumThreadResult = "shared" | "copied" | "cancelled";

export function forumThreadUrl(threadId: string) {
  const basePath = import.meta.env.BASE_URL.endsWith("/")
    ? import.meta.env.BASE_URL
    : `${import.meta.env.BASE_URL}/`;
  return new URL(
    `${basePath}forum/threads/${encodeURIComponent(threadId)}`,
    window.location.origin,
  ).toString();
}

export async function shareForumThread(
  title: string,
  threadId: string,
): Promise<ShareForumThreadResult> {
  const url = forumThreadUrl(threadId);
  if (navigator.share) {
    try {
      await navigator.share({ title, url });
      return "shared";
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return "cancelled";
    }
  }
  if (!navigator.clipboard?.writeText) {
    throw new Error("当前浏览器不支持分享或复制链接");
  }
  await navigator.clipboard.writeText(url);
  return "copied";
}
