<script lang="ts">
  // Shows whichever window `ui.editing` points at (settings, preferences, about, add from URL).
  // Scenes, presets and sounds are edited in place instead (Workspace, LibraryWorkspace).
  import { ui } from "../lib/store.svelte";
  import About from "./About.svelte";
  import AddFromUrl from "./AddFromUrl.svelte";
  import PreferencesEditor from "./PreferencesEditor.svelte";
  import ProjectSettings from "./ProjectSettings.svelte";

  const close = () => (ui.editing = null);
</script>

{#if ui.editing?.kind === "projectSettings" && ui.snapshot?.project}
  <ProjectSettings onclose={close} />
{:else if ui.editing?.kind === "preferences"}
  <PreferencesEditor onclose={close} page={ui.editing.page} />
{:else if ui.editing?.kind === "about"}
  <About onclose={close} />
{:else if ui.editing?.kind === "addUrl" && ui.snapshot?.project}
  <AddFromUrl onclose={close} />
{/if}
