<script lang="ts">
  import { tick } from "svelte";
  import { X } from "@lucide/svelte";
  import IconButton from "./IconButton.svelte";

  interface Props {
    open?: boolean;
    title?: string;
    showCloseButton?: boolean;
    closeOnBackdropClick?: boolean;
    onClose?: () => void;
    children?: import("svelte").Snippet;
  }

  let {
    open = $bindable(false),
    title = "",
    showCloseButton = true,
    closeOnBackdropClick = false,
    onClose = () => {},
    children,
  }: Props = $props();

  let closing = $state(false);
  let modalElement = $state<HTMLDivElement>();

  export function handleClose() {
    closing = true;
    setTimeout(() => {
      closing = false;
      onClose();
    }, 250);
  }

  function handleBackdropClick(e: MouseEvent) {
    if (closeOnBackdropClick && e.target === e.currentTarget) {
      handleClose();
    }
  }

  $effect(() => {
    if (open && modalElement) {
      tick().then(() => modalElement?.focus());
    }
  });
</script>

{#if open}
  <div
    bind:this={modalElement}
    class="fixed inset-0 flex items-center justify-center z-[10010] animate-backdrop"
    style="background-color: var(--overlay);"
    class:animate-backdrop-close={closing}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={handleBackdropClick}
    onkeydown={(e) => {
      if (e.key === "Escape") handleClose();
    }}
  >
    <!-- Modal container -->
    <div class="relative animate-modal" class:animate-modal-close={closing}>
      <!-- Background layer: responsible for visual effects and border clipping -->
      <div
        class="bg-[var(--base-100)] max-w-[90vw] max-h-[90vh] rounded-2xl shadow-2xl overflow-hidden relative pointer-events-none border-[1px] border-[var(--base-200)]"
        style="border-radius: 20px; z-index: 1;"
      >
        <!-- Reserve content space -->
        <div class="px-0 py-0 invisible">
          {#if children}
            {@render children()}
          {/if}
        </div>
      </div>

      <!-- Content layer: independent of background layer, not affected by clipping -->
      <div
        class="absolute inset-0 max-w-4xl bg-transparent"
        style="z-index: 2;"
      >
        <!-- Header with close button and title -->
        {#if showCloseButton || title}
          <div
            class="absolute top-0 left-0 right-0 z-20 flex items-center justify-between px-5 py-4"
          >
            {#if title}
              <h3 class="text-base font-medium text-base-content">
                {title}
              </h3>
            {/if}
            {#if showCloseButton}
              <IconButton onclick={handleClose} ariaLabel="Close" title="Close">
                <X size={20} />
              </IconButton>
            {/if}
          </div>
        {/if}

        <!-- Content area: not affected by background layer clipping -->
        <div class="px-0 py-0">
          {#if children}
            {@render children()}
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .animate-backdrop {
    animation: backdropFadeIn 0.25s ease-out;
  }

  .animate-backdrop-close {
    animation: backdropFadeOut 0.25s ease-out;
  }

  .animate-modal {
    animation: modalSlideIn 0.25s ease-out;
  }

  .animate-modal-close {
    animation: modalSlideOut 0.25s ease-out;
  }

  @keyframes backdropFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes backdropFadeOut {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-30px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes modalSlideOut {
    from {
      opacity: 1;
      transform: translateY(0);
    }
    to {
      opacity: 0;
      transform: translateY(-30px);
    }
  }
</style>
