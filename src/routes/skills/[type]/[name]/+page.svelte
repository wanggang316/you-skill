<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { get } from "svelte/store";
  import { ChevronLeft, Loader2 } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SkillFileViewer, { type FileSource } from "$lib/components/SkillFileViewer.svelte";
  import { t } from "$lib/i18n";
  import { fetchSkillsByNames } from "$lib/api/skills";
  import { getHubSkill } from "$lib/api/hub";

  let loading = $state(true);
  let error = $state("");
  let source = $state<FileSource | null>(null);
  let currentName = $state("");

  const params = $derived($page.params);

  const decodeName = (value: string): string => {
    try {
      return decodeURIComponent(value);
    } catch {
      return value;
    }
  };

  const loadSkill = async () => {
    loading = true;
    error = "";
    source = null;
    const name = decodeName(params.name);
    currentName = name;
    try {
      if (params.type === "remote") {
        const remoteSkills = await fetchSkillsByNames([name]);
        const skill = remoteSkills.find((item) => item.name === name) ?? remoteSkills[0] ?? null;
        source = skill ? { kind: "remote", name, skill } : null;
      } else if (params.type === "hub" || params.type === "local") {
        const skill = await getHubSkill(name);
        source = skill ? { kind: "hub", name, rootPath: skill.hubPath } : null;
      } else {
        error = "Invalid skill type";
        return;
      }
      if (!source) {
        error = $t("detail.notFound");
      }
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  };

  const handleBack = () => {
    const returnTo = get(page).url.searchParams.get("returnTo");
    if (returnTo) {
      goto(decodeURIComponent(returnTo));
      return;
    }
    window.history.back();
  };

  $effect(() => {
    params.type;
    params.name;
    loadSkill().catch(console.error);
  });
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <header
    class="border-base-300 flex h-12 flex-none items-center gap-3 border-b px-6"
    data-window-drag-region
  >
    <IconButton
      variant="outline"
      onclick={handleBack}
      title={$t("header.back")}
      class="h-8 w-8 p-0"
    >
      <ChevronLeft size={16} />
    </IconButton>
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {currentName}
    </h1>
  </header>

  {#if loading}
    <div class="flex flex-1 items-center justify-center py-12">
      <Loader2 size={32} class="text-base-content-muted animate-spin" />
    </div>
  {:else if error || !source}
    <div class="p-6">
      <div class="border-error-border bg-base-100 rounded-2xl border p-6 text-center">
        <p class="text-error">{error || $t("detail.notFound")}</p>
      </div>
    </div>
  {:else}
    <SkillFileViewer {source} />
  {/if}
</section>
