<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { goto } from "$app/navigation";
  import AppSidebar from "$lib/components/AppSidebar.svelte";
  import { listUserProjects, type UserProject } from "$lib/api/user-projects";
  import { ensureUpdateChecked, installAvailableUpdate, updaterState } from "$lib/stores/updater";

  let {
    activeTab = "local",
    currentSection = "skills",
    initialScopeKey = "global",
    children,
  }: {
    activeTab?: "local" | "remote";
    currentSection?: "skills" | "settings";
    initialScopeKey?: string;
    children: Snippet;
  } = $props();

  let scopeKey = $state("global");
  let projects = $state<UserProject[]>([]);
  let hasUpdate = $state(false);
  let updateLoading = $state(false);

  $effect(() => {
    scopeKey = initialScopeKey;
  });

  onMount(() => {
    const unsubscribeUpdater = updaterState.subscribe((state) => {
      hasUpdate = state.hasUpdate;
      updateLoading = state.installing;
    });

    listUserProjects()
      .then((items) => {
        projects = items;
      })
      .catch(console.error);

    ensureUpdateChecked().catch(console.error);

    return unsubscribeUpdater;
  });

  const navigateToSkills = (tab: "local" | "remote", selectedScopeKey = "global") => {
    const params = new URLSearchParams({ tab });
    if (tab === "local" && selectedScopeKey.startsWith("project:")) {
      params.set("scope", "project");
      params.set("projectPath", decodeURIComponent(selectedScopeKey.slice("project:".length)));
    } else {
      params.set("scope", "global");
    }
    goto(`/?${params.toString()}`);
  };
</script>

<div
  class="bg-base-100 text-base-content grid h-screen grid-cols-[15.5rem_minmax(0,1fr)] overflow-hidden max-[832px]:grid-cols-[13.5rem_minmax(0,1fr)]"
>
  <AppSidebar
    {activeTab}
    {currentSection}
    bind:scopeKey
    {projects}
    {hasUpdate}
    {updateLoading}
    onAddSkill={() => goto("/?action=add")}
    onSelectTab={navigateToSkills}
    onOpenUpdate={() => installAvailableUpdate().catch(console.error)}
    onOpenProjectManage={() => goto("/?action=manage-projects")}
    onOpenSettings={() => goto("/settings")}
  />

  <section class="flex min-w-0 flex-col overflow-hidden">
    {@render children()}
  </section>
</div>
