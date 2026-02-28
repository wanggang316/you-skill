<script lang="ts">
  let {
    htmlContent = "",
    frontmatterDescription = "",
    onOpenExternalLink,
    onOpenRelativeLink,
  }: {
    htmlContent?: string;
    frontmatterDescription?: string;
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

{#if frontmatterDescription}
  <div class="border-base-300 bg-base-200 mb-6 overflow-hidden rounded-xl border">
    <div class="flex flex-col gap-3 p-4">
      <div class="flex flex-col gap-1">
        <span class="text-base-content text-sm leading-6">{frontmatterDescription}</span>
      </div>
    </div>
  </div>
{/if}

<div class="markdown-content" use:bindInteractions>
  {@html htmlContent}
</div>
