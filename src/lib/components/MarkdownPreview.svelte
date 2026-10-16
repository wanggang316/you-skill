<script lang="ts">
  import type { TranslateTextStyle } from "$lib/api/settings";
  import FrontmatterTable from "./FrontmatterTable.svelte";

  let {
    htmlContent = "",
    frontmatter = {},
    translatedDescription = "",
    translationStyle = "none",
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
    onOpenExternalLink?: (href: string) => Promise<void> | void;
    onOpenRelativeLink?: (href: string) => Promise<void> | void;
  } = $props();

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
    if (!href || href.startsWith("#")) return;

    event.preventDefault();
    event.stopPropagation();

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
</script>

<div class:markdown-translation-dashed={translationStyle === "dashed_underline"}>
  <FrontmatterTable {frontmatter} {translatedDescription} {onOpenExternalLink} />

  <div class="markdown-content" use:bindInteractions>
    {@html htmlContent}
  </div>
</div>
