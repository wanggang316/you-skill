<script lang="ts">
  import ampIcon from "@lobehub/icons-static-svg/icons/amp-color.svg";
  import antigravityIcon from "@lobehub/icons-static-svg/icons/antigravity-color.svg";
  import claudeCodeIcon from "@lobehub/icons-static-svg/icons/claudecode-color.svg";
  import clineIcon from "@lobehub/icons-static-svg/icons/cline.svg";
  import codeBuddyIcon from "@lobehub/icons-static-svg/icons/codebuddy-color.svg";
  import codexIcon from "@lobehub/icons-static-svg/icons/codex-color.svg";
  import cursorIcon from "@lobehub/icons-static-svg/icons/cursor.svg";
  import geminiCliIcon from "@lobehub/icons-static-svg/icons/geminicli-color.svg";
  import githubCopilotIcon from "@lobehub/icons-static-svg/icons/githubcopilot.svg";
  import gooseIcon from "@lobehub/icons-static-svg/icons/goose.svg";
  import junieIcon from "@lobehub/icons-static-svg/icons/junie-color.svg";
  import kiloCodeIcon from "@lobehub/icons-static-svg/icons/kilocode.svg";
  import kimiIcon from "@lobehub/icons-static-svg/icons/kimi-color.svg";
  import kiroIcon from "@lobehub/icons-static-svg/icons/kiro-color.svg";
  import mistralIcon from "@lobehub/icons-static-svg/icons/mistral-color.svg";
  import openClawIcon from "@lobehub/icons-static-svg/icons/openclaw-color.svg";
  import openCodeIcon from "@lobehub/icons-static-svg/icons/opencode.svg";
  import openHandsIcon from "@lobehub/icons-static-svg/icons/openhands-color.svg";
  import piIcon from "@lobehub/icons-static-svg/icons/pi.svg";
  import qoderIcon from "@lobehub/icons-static-svg/icons/qoder-color.svg";
  import qwenIcon from "@lobehub/icons-static-svg/icons/qwen-color.svg";
  import replitIcon from "@lobehub/icons-static-svg/icons/replit-color.svg";
  import rooCodeIcon from "@lobehub/icons-static-svg/icons/roocode.svg";
  import traeIcon from "@lobehub/icons-static-svg/icons/trae-color.svg";
  import windsurfIcon from "@lobehub/icons-static-svg/icons/windsurf.svg";
  import zencoderIcon from "@lobehub/icons-static-svg/icons/zencoder-color.svg";
  import zenMuxIcon from "@lobehub/icons-static-svg/icons/zenmux.svg";

  type AgentIcon = {
    src: string;
    monochrome?: boolean;
  };

  const icons: Record<string, AgentIcon> = {
    "claude-code": { src: claudeCodeIcon },
    codex: { src: codexIcon },
    cursor: { src: cursorIcon, monochrome: true },
    cline: { src: clineIcon, monochrome: true },
    opencode: { src: openCodeIcon, monochrome: true },
    openhands: { src: openHandsIcon },
    "github-copilot": { src: githubCopilotIcon, monochrome: true },
    "gemini-cli": { src: geminiCliIcon },
    goose: { src: gooseIcon, monochrome: true },
    windsurf: { src: windsurfIcon, monochrome: true },
    roo: { src: rooCodeIcon, monochrome: true },
    "kiro-cli": { src: kiroIcon },
    "qwen-code": { src: qwenIcon },
    amp: { src: ampIcon },
    antigravity: { src: antigravityIcon },
    trae: { src: traeIcon },
    "trae-cn": { src: traeIcon },
    openclaw: { src: openClawIcon },
    codebuddy: { src: codeBuddyIcon },
    junie: { src: junieIcon },
    kilo: { src: kiloCodeIcon, monochrome: true },
    "kimi-cli": { src: kimiIcon },
    "mistral-vibe": { src: mistralIcon },
    mux: { src: zenMuxIcon, monochrome: true },
    pi: { src: piIcon, monochrome: true },
    qoder: { src: qoderIcon },
    replit: { src: replitIcon },
    zenencoder: { src: zencoderIcon },
  };

  let { agentId, name }: { agentId: string; name: string } = $props();

  const icon = $derived(icons[agentId]);
  const fallback = $derived(name.trim().charAt(0).toUpperCase() || "?");
</script>

<span class:monochrome={icon?.monochrome} class="agent-app-icon" aria-hidden="true">
  {#if icon}
    <img src={icon.src} alt="" />
  {:else}
    <span class="fallback">{fallback}</span>
  {/if}
</span>

<style>
  .agent-app-icon {
    display: inline-flex;
    width: 2rem;
    height: 2rem;
    flex: none;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--base-300);
    border-radius: 0.65rem;
    background: var(--base-100);
  }

  .agent-app-icon img {
    width: 1.15rem;
    height: 1.15rem;
    object-fit: contain;
  }

  .fallback {
    color: var(--base-content-subtle);
    font-size: 0.78rem;
    font-weight: 650;
    line-height: 1;
  }

  :global([data-theme="dark"]) .agent-app-icon.monochrome img {
    filter: invert(1);
  }
</style>
