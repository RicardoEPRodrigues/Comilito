<script lang="ts">
  // Project settings: the defaults every scene and sound in this project inherits.
  // Stored in comilito.json, so the project sounds the same on every computer.
  import { api } from "../lib/api";
  import { ask, project, run } from "../lib/store.svelte";
  import type { BackupSettings, ProjectDefaults } from "../lib/types";
  import DefaultsForm from "./DefaultsForm.svelte";
  import Modal from "./Modal.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let defaults = $state<ProjectDefaults>(structuredClone($state.snapshot(project()!.defaults)));
  let backups = $state<BackupSettings>(structuredClone($state.snapshot(project()!.backups)));

  async function save() {
    await run(api.setProjectDefaults($state.snapshot(defaults)));
    await run(api.setBackupSettings($state.snapshot(backups)));
    onclose();
  }

  async function resetToApp() {
    const answer = await ask({
      title: "Reset to my app defaults?",
      message: "Replaces this project's levels, sound defaults and backup settings with the ones in Preferences.",
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Reset", value: "reset", primary: true },
      ],
    });
    if (answer?.choice !== "reset") return;
    await run(api.resetProjectDefaults());
    onclose();
  }
</script>

<Modal title="Project settings" {onclose}>
  <p class="muted">
    Scenes and sounds use these unless they set their own value. Changing one here updates
    everything that inherits it.
  </p>
  <DefaultsForm bind:defaults bind:backups />

  {#snippet footer()}
    <button onclick={resetToApp}>Reset to my app defaults</button>
    <span style="flex: 1"></span>
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save}>Save</button>
  {/snippet}
</Modal>

<style>
  p {
    margin: 0;
  }
</style>
