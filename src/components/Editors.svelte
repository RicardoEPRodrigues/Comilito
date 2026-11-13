<script lang="ts">
  // Shows whichever editor `ui.editing` points at.
  import { sceneById, soundById, ui } from "../lib/store.svelte";
  import About from "./About.svelte";
  import PreferencesEditor from "./PreferencesEditor.svelte";
  import ProjectSettings from "./ProjectSettings.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import SoundEditor from "./SoundEditor.svelte";

  const close = () => (ui.editing = null);
</script>

{#if ui.editing?.kind === "scene"}
  {@const scene = sceneById(ui.editing.id)}
  {#if scene}
    <!-- `{#key}` recreates the editor when switching to another scene, resetting its draft. -->
    {#key scene.id}
      <SceneEditor original={scene} onclose={close} />
    {/key}
  {/if}
{:else if ui.editing?.kind === "sound"}
  {@const sound = soundById(ui.editing.id)}
  {#if sound}
    {#key sound.id}
      <SoundEditor original={sound} onclose={close} />
    {/key}
  {/if}
{:else if ui.editing?.kind === "projectSettings" && ui.snapshot?.project}
  <ProjectSettings onclose={close} />
{:else if ui.editing?.kind === "preferences"}
  <PreferencesEditor onclose={close} />
{:else if ui.editing?.kind === "about"}
  <About onclose={close} />
{/if}
