<script lang="ts" module>
  // "Save the current tapestry?" (decision 0017). Shown before the tapestry is replaced while
  // it isn't saved in a preset, and for "Save as new preset…" from the tapestry's menu.
  //
  // `askSaveTapestry(target)` opens it and resolves when it closes:
  //   "saved"   – saved (as a new preset or by updating its origin)
  //   "replace" – replace without saving
  //   "cancel"  – keep the tapestry, do nothing
  // With `target = null` it's the plain "Save as new preset" form (no replace option).
  type Answer = "saved" | "replace" | "cancel";

  export const saveRequest = $state({
    open: false,
    target: null as string | null,
    resolve: (_: Answer) => {},
  });

  export function askSaveTapestry(target: string | null): Promise<Answer> {
    return new Promise((resolve) => {
      saveRequest.target = target;
      saveRequest.resolve = resolve;
      saveRequest.open = true;
    });
  }
</script>

<script lang="ts">
  import { backdrop, rise } from "../lib/motion";
  import { BookmarkPlus, Check } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { presetById, project, run, savePrefs, sceneById, ui } from "../lib/store.svelte";

  const p = $derived(project());
  const origin = $derived(p?.stackOrigin ? presetById(p.stackOrigin) : undefined);
  const canUpdate = $derived(ui.snapshot?.stackStatus === "modified" && !!origin);
  const groups = $derived(ui.snapshot?.presetGroups ?? []);

  let choice = $state<"update" | "new">("new");
  let name = $state("");
  let group = $state("");
  let dontAsk = $state(false);
  let saving = $state(false);

  // Fresh values each time the dialog opens.
  $effect(() => {
    if (!saveRequest.open) return;
    choice = canUpdate ? "update" : "new";
    name = origin ? `${origin.name}, take 2` : "";
    group = origin?.group ?? "";
    dontAsk = false;
  });

  function close(answer: Answer) {
    saveRequest.open = false;
    saveRequest.resolve(answer);
  }

  async function save() {
    if (saving) return;
    saving = true;
    let ok: unknown;
    if (choice === "update" && origin) {
      ok = await run(api.presetUpdate(origin.id).then(() => true));
    } else {
      if (!name.trim()) {
        saving = false;
        return;
      }
      ok = await run(api.presetSave(name.trim(), group.trim() || null));
    }
    saving = false;
    if (ok) close("saved");
  }

  async function replace() {
    if (dontAsk && ui.prefs) {
      await savePrefs({ ...ui.prefs, behaviour: { ...ui.prefs.behaviour, replaceUnsavedTapestry: "replace" } });
    }
    close("replace");
  }

  function onKeydown(e: KeyboardEvent) {
    if (!saveRequest.open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      close("cancel");
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if saveRequest.open && p}
  <div class="backdrop" transition:backdrop>
    <div class="dialog" transition:rise role="dialog" aria-modal="true" aria-label="Save the tapestry">
      <div class="heading">
        <BookmarkPlus size={20} />
        <h3 class="display">{saveRequest.target ? "Save the current tapestry?" : "Save as a preset"}</h3>
      </div>
      {#if saveRequest.target}
        <p class="muted">
          Loading {saveRequest.target} replaces the tapestry.
          {#if canUpdate}It started from “{origin?.name}” but has changed since.{:else}What's playing now isn't saved in any preset.{/if}
        </p>
      {/if}

      <div class="current">
        <span class="caps muted">Current tapestry</span>
        {#each [...p.stack].reverse() as entry (entry.id)}
          <div class="entry" class:off={!entry.enabled}>
            <span class="box" class:on={entry.enabled}>{#if entry.enabled}<Check size={10} />{/if}</span>
            {sceneById(entry.scene)?.name ?? "?"}
          </div>
        {/each}
      </div>

      {#if canUpdate}
        <label class="option" class:selected={choice === "update"}>
          <input type="radio" bind:group={choice} value="update" />
          <span>
            <strong>Update “{origin?.name}”</strong>
            <span class="muted small">Overwrite the preset with what's playing now</span>
          </span>
        </label>
      {/if}
      <div class="option col" class:selected={choice === "new"}>
        <label class="row">
          {#if canUpdate}<input type="radio" bind:group={choice} value="new" />{/if}
          <span>
            <strong>Save as a new preset</strong>
            {#if canUpdate}<span class="muted small">Keep “{origin?.name}” as it was</span>{/if}
          </span>
        </label>
        <div class="fields" class:dim={choice !== "new"}>
          <label class="field grow">
            <span class="caps">Preset name</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input bind:value={name} disabled={choice !== "new"} autofocus={choice === "new"}
              onkeydown={(e) => e.key === "Enter" && save()} />
          </label>
          <label class="field">
            <span class="caps">Group</span>
            <input bind:value={group} list="preset-groups" placeholder="Ungrouped" disabled={choice !== "new"} />
            <datalist id="preset-groups">
              {#each groups as g (g)}<option value={g}></option>{/each}
            </datalist>
          </label>
        </div>
      </div>

      {#if saveRequest.target}
        <label class="dont-ask">
          <input type="checkbox" bind:checked={dontAsk} />
          <span class="muted small">Don't ask again (change it in Preferences › Behaviour)</span>
        </label>
      {/if}

      <div class="buttons">
        <button class="ghost" onclick={() => close("cancel")}>Cancel</button>
        <span class="spacer"></span>
        {#if saveRequest.target}
          <button onclick={replace}>Replace without saving</button>
        {/if}
        <button class="primary" onclick={save} disabled={saving || (choice === "new" && !name.trim())}>
          <Check size={14} />{saveRequest.target ? "Save and load" : "Save"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .dialog {
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 1.4rem;
    width: min(34rem, 92vw);
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .heading {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--accent);
  }
  h3 {
    color: var(--text);
    font-size: 1.7rem;
  }
  p {
    margin: 0;
    line-height: 1.45;
  }
  .small {
    display: block;
    font-size: 0.78rem;
  }
  .current {
    background: var(--bg);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    padding: 0.7rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 9rem;
    overflow: auto;
  }
  .current .caps {
    font-size: 0.6rem;
    margin-bottom: 0.15rem;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .entry.off {
    opacity: 0.5;
  }
  .box {
    width: 0.8rem;
    height: 0.8rem;
    border: 1.2px solid var(--text);
    border-radius: 2px;
    display: grid;
    place-items: center;
  }
  .box.on {
    background: var(--text);
    color: var(--surface);
  }
  .option {
    display: flex;
    gap: 0.7rem;
    align-items: center;
    padding: 0.8rem;
    border: 1px solid var(--rule);
    border-radius: 6px;
    cursor: pointer;
  }
  .option.col {
    flex-direction: column;
    align-items: stretch;
    cursor: default;
  }
  .option.selected {
    border-color: var(--accent);
    border-width: 1.5px;
  }
  .row {
    display: flex;
    gap: 0.7rem;
    align-items: center;
    cursor: pointer;
  }
  .fields {
    display: flex;
    gap: 0.7rem;
  }
  .fields.dim {
    opacity: 0.5;
  }
  .grow {
    flex: 1;
  }
  .dont-ask {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .buttons {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .spacer {
    flex: 1;
  }
</style>
