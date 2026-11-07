<script lang="ts">
  import { Portal } from "bits-ui";
  import type { TranslateTextStyle } from "$lib/api/settings";
  import { linkAnchor, linkPreviewText } from "$lib/utils/markdown-links";
  import FrontmatterTable from "./FrontmatterTable.svelte";

  let {
    htmlContent = "",
    frontmatter = {},
    translatedDescription = "",
    translationStyle = "none",
    filePath,
    anchor = "",
    onAnchorHandled,
    onOpenExternalLink,
    onOpenRelativeLink,
  }: {
    htmlContent?: string;
    /** Frontmatter fields shown as a table above the body, in document order. */
    frontmatter?: Record<string, unknown>;
    /** Shown under the `description` field in bilingual view. */
    translatedDescription?: string;
    /** Style of `.markdown-translation` elements in bilingual view. */
    translationStyle?: TranslateTextStyle;
    /** Path of the shown file in the skill; the Link preview resolves relative links against it. */
    filePath?: string;
    /** Section to jump to once the content renders, e.g. after a link to `other.md#section`. */
    anchor?: string;
    onAnchorHandled?: () => void;
    onOpenExternalLink?: (href: string) => Promise<void> | void;
    onOpenRelativeLink?: (href: string) => Promise<void> | void;
  } = $props();

  // Room above a jump target, so the heading does not touch the top edge.
  const ANCHOR_TOP_MARGIN = 16;

  let root = $state<HTMLElement>();
  let hoveredHref = $state<string | null>(null);
  const linkPreview = $derived(hoveredHref === null ? "" : linkPreviewText(hoveredHref, filePath));

  /** Body links are `<a href>`; frontmatter links are buttons with `data-link-href`. */
  const linkHrefAt = (target: EventTarget | null): string | null => {
    if (!(target instanceof Element)) return null;
    const link = target.closest("a[href], [data-link-href]");
    if (!link || !root?.contains(link)) return null;
    return link.getAttribute("data-link-href") ?? link.getAttribute("href");
  };

  const findAnchorTarget = (name: string): HTMLElement | undefined => {
    if (!root || !name) return undefined;
    const byId = [...root.querySelectorAll<HTMLElement>("[id]")].find((el) => el.id === name);
    return (
      byId ??
      [...root.querySelectorAll<HTMLElement>("a[name]")].find(
        (el) => el.getAttribute("name") === name
      )
    );
  };

  const scrollContainerOf = (element: HTMLElement): HTMLElement | null => {
    for (let node = element.parentElement; node; node = node.parentElement) {
      const { overflowY } = getComputedStyle(node);
      if (
        (overflowY === "auto" || overflowY === "scroll") &&
        node.scrollHeight > node.clientHeight
      ) {
        return node;
      }
    }
    return null;
  };

  /** Scrolls the preview to a section. The URL hash and history stay unchanged. */
  const jumpToAnchor = (name: string) => {
    const target = findAnchorTarget(name);
    if (!target) {
      console.warn(`Markdown link target not found: #${name}`);
      return;
    }
    const container = scrollContainerOf(target);
    if (!container) {
      target.scrollIntoView({ behavior: "smooth", block: "start" });
      return;
    }
    const offset = target.getBoundingClientRect().top - container.getBoundingClientRect().top;
    container.scrollTo({
      top: Math.max(0, container.scrollTop + offset - ANCHOR_TOP_MARGIN),
      behavior: "smooth",
    });
  };

  $effect(() => {
    // The link under the pointer can go away with the old content.
    void htmlContent;
    hoveredHref = null;
  });

  $effect(() => {
    if (!anchor) return;
    // Effects run after the DOM update, so the new content is in place.
    void htmlContent;
    jumpToAnchor(anchor);
    onAnchorHandled?.();
  });

  const handleClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    const button = target.closest(".markdown-code-block__copy") as HTMLElement | null;
    if (button) {
      event.preventDefault();
      event.stopPropagation();
      const block = button.closest(".markdown-code-block");
      const codeElement = block?.querySelector("code");
      const codeContent = codeElement?.textContent ?? "";
      if (!codeContent) return;
      try {
        await navigator.clipboard.writeText(codeContent);
      } catch {
        const textarea = document.createElement("textarea");
        textarea.value = codeContent;
        textarea.setAttribute("readonly", "");
        textarea.style.position = "absolute";
        textarea.style.left = "-9999px";
        document.body.appendChild(textarea);
        textarea.select();
        try {
          document.execCommand("copy");
        } catch {
          // noop
        }
        document.body.removeChild(textarea);
      }
      button.classList.add("copied");
      const timerId = button.dataset.copyTimeout;
      if (timerId) window.clearTimeout(Number(timerId));
      const timeoutHandle = window.setTimeout(() => {
        button.classList.remove("copied");
        delete button.dataset.copyTimeout;
      }, 1500);
      button.dataset.copyTimeout = String(timeoutHandle);
      return;
    }

    const link = target.closest("a[href]");
    if (!link) return;
    const href = link.getAttribute("href");
    if (!href) return;

    event.preventDefault();
    event.stopPropagation();

    if (href.startsWith("#")) {
      jumpToAnchor(linkAnchor(href));
      return;
    }

    if (href.startsWith("http://") || href.startsWith("https://")) {
      await onOpenExternalLink?.(href);
      return;
    }

    await onOpenRelativeLink?.(href);
  };

  const bindInteractions = (node: HTMLElement) => {
    node.addEventListener("click", handleClick);
    return {
      destroy() {
        node.removeEventListener("click", handleClick);
      },
    };
  };

  const handlePointerOver = (event: PointerEvent) => {
    hoveredHref = linkHrefAt(event.target);
  };

  const handlePointerLeave = () => {
    hoveredHref = null;
  };

  /** Tracks the link under the pointer in the body and the frontmatter table. */
  const trackLinkHover = (node: HTMLElement) => {
    node.addEventListener("pointerover", handlePointerOver);
    node.addEventListener("pointerleave", handlePointerLeave);
    return {
      destroy() {
        node.removeEventListener("pointerover", handlePointerOver);
        node.removeEventListener("pointerleave", handlePointerLeave);
      },
    };
  };
</script>

<div
  bind:this={root}
  class:markdown-translation-dashed={translationStyle === "dashed_underline"}
  use:trackLinkHover
>
  <FrontmatterTable {frontmatter} {translatedDescription} {onOpenExternalLink} />

  <div class="markdown-content" use:bindInteractions>
    {@html htmlContent}
  </div>
</div>

{#if linkPreview}
  <!-- On `body`, so the text stays at the window corner also when the preview is in a dialog. -->
  <Portal>
    <div
      class="border-base-300 bg-base-200 text-base-content-muted pointer-events-none fixed bottom-0 left-0 z-(--z-popover) max-w-[min(60vw,40rem)] truncate rounded-tr-md border-t border-r px-2 py-0.5 text-xs"
      role="status"
    >
      {linkPreview}
    </div>
  </Portal>
{/if}
