<script lang="ts">
  // "Add Comilito to the app menu?" (decision 0065). Shown at start when the AppImage isn't in
  // the desktop's app menu, or its entry needs updating, unless turned off; and from
  // Preferences › Projects & tools. Nothing changes without a yes here.
  import { AppWindow } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { savePrefs, ui } from "../lib/store.svelte";
  import Modal from "./Modal.svelte";

  const status = $derived(ui.appMenu!);
  const fromStart = ui.appMenuOffer === "start";
  const adding = $derived(status.state === "missing");
  const fileName = $derived(status.appimage?.split("/").pop() ?? "");

  // Moving out of Downloads (or a temporary folder) is the likely wish: those get cleaned.
  // svelte-ignore state_referenced_locally
  let moveIt = $state(!status.inApplicationsDir && /\/Downloads\/|^\/tmp\//.test(status.appimage ?? ""));
  let dontAsk = $state(false);
  let busy = $state(false);
  let error = $state("");

  const title = $derived(adding ? "Add Comilito to the app menu?" : "Update Comilito in the app menu?");

  async function rememberChoice() {
    if (dontAsk && ui.prefs) await savePrefs({ ...ui.prefs, behaviour: { ...ui.prefs.behaviour, askAppMenu: false } });
  }

  async function accept() {
    busy = true;
    error = "";
    try {
      ui.appMenu = await api.addToAppMenu(moveIt && !status.inApplicationsDir);
      await rememberChoice();
      ui.appMenuOffer = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function notNow() {
    await rememberChoice();
    ui.appMenuOffer = null;
  }
</script>

<Modal {title} onclose={notNow}>
  <div class="body">
    {#if status.state === "missing"}
      <p>
        Comilito isn't in your desktop's app menu. Adding it puts it there with its icon, so you can start it like any
        other app. It adds a launcher and icons to your data folder (~/.local/share); nothing else changes.
      </p>
    {:else if status.state === "moved"}
      <p>
        The app menu's Comilito starts <code>{status.pointsAt}</code>, which isn't there any more (moved, renamed or
        deleted). Point it at this copy instead?
      </p>
    {:else if status.state === "otherCopy"}
      <p>The app menu's Comilito starts another copy, <code>{status.pointsAt}</code>. Start this one from the menu instead?</p>
    {:else}
      <p>This version of Comilito comes with a different app menu launcher or icon. Update them?</p>
    {/if}
    <p class="muted small"><AppWindow size={13} />This copy: <code>{status.appimage}</code></p>

    {#if !status.inApplicationsDir}
      <label class="check">
        <input type="checkbox" bind:checked={moveIt} />
        <span>
          Move it to <code>{status.applicationsDir}</code> first
          <span class="muted small block">
            The menu stops working if the file is moved or deleted, as Downloads often is.
            {#if status.replacesInApplicationsDir}The {fileName} already there goes to the trash.{/if}
          </span>
        </span>
      </label>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  {#snippet footer()}
    {#if fromStart}
      <label class="check grow">
        <input type="checkbox" bind:checked={dontAsk} />
        <span>Don't ask again <span class="muted small">(Preferences › Projects & tools)</span></span>
      </label>
    {:else}
      <span class="grow"></span>
    {/if}
    <button class="ghost" onclick={notNow} disabled={busy}>Not now</button>
    <button class="primary" onclick={accept} disabled={busy}>{adding ? "Add to the app menu" : "Update"}</button>
  {/snippet}
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    max-width: 36rem;
  }
  p {
    margin: 0;
    line-height: 1.45;
  }
  p.small {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  code {
    font-size: 0.82rem;
    user-select: text;
    overflow-wrap: anywhere;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
  }
  .check input {
    margin-top: 0.2rem;
  }
  .block {
    display: block;
    margin-top: 0.15rem;
  }
  .small {
    font-size: 0.78rem;
  }
  .grow {
    flex: 1;
  }
  .error {
    color: var(--danger);
  }
</style>
