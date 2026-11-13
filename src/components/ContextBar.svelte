<script lang="ts">
  // The gold bar under the header while editing something in place or browsing the Library
  // (decision 0017): breadcrumbs that say where you are, plus the editor's Discard / Save.
  // Clicking a crumb goes back to that level.
  import { ArrowLeft, Check, ChevronRight, FolderOpen, Layers, Library, Music, PanelsTopLeft, Scissors, Scroll } from "@lucide/svelte";
  import type { Component } from "svelte";
  import { focusOn, leaveFocus, presetById, sceneById, showView, soundById, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";

  interface Crumb {
    label: string;
    icon: Component;
    go?: () => void;
  }

  const lib = $derived(ui.library);
  const crumbs = $derived.by((): Crumb[] => {
    if (ui.view === "library") {
      const list: Crumb[] = [
        {
          label: "Library",
          icon: Library,
          go: () => {
            lib.cropping = false;
            lib.smart = "all";
            lib.category = null;
            lib.folder = null;
          },
        },
      ];
      if (lib.category) {
        const label = CATEGORIES.find((c) => c.id === lib.category)?.label ?? "";
        list.push({ label, icon: Music, go: () => ((lib.cropping = false), (lib.folder = null)) });
        if (lib.folder) {
          let path = "";
          for (const part of lib.folder.split("/")) {
            path = path ? `${path}/${part}` : part;
            const target = path;
            list.push({ label: part, icon: FolderOpen, go: () => ((lib.cropping = false), (lib.folder = target)) });
          }
        }
      }
      const one = lib.selected.length === 1 ? soundById(lib.selected[0]) : undefined;
      if (one) list.push({ label: one.name, icon: Music, go: () => (lib.cropping = false) });
      else if (lib.selected.length > 1) list.push({ label: `${lib.selected.length} sounds`, icon: Music });
      if (lib.cropping) list.push({ label: "Crop", icon: Scissors });
      return list;
    }
    const list: Crumb[] = [{ label: "Tapestry", icon: PanelsTopLeft, go: () => leaveFocus() }];
    const focus = ui.focus;
    if (focus?.kind === "preset") {
      list.push({ label: presetById(focus.id)?.name ?? "Preset", icon: Layers });
    } else if (focus?.kind === "scene") {
      if (focus.from) {
        const from = focus.from;
        list.push({ label: presetById(from)?.name ?? "Preset", icon: Layers, go: () => focusOn({ kind: "preset", id: from }) });
      }
      list.push({ label: sceneById(focus.id)?.name ?? "Scene", icon: Scroll });
    }
    return list;
  });

  const note = $derived(
    ui.view === "library" ? "The tapestry keeps playing · Esc to go back" : "Editing in place · the tapestry keeps playing unchanged",
  );

  function back() {
    if (ui.view === "library") {
      if (lib.cropping) lib.cropping = false;
      else showView("tapestry");
    } else leaveFocus();
  }
</script>

<div class="bar">
  <button class="back" onclick={back}><ArrowLeft size={14} />{ui.view === "library" && !lib.cropping ? "Tapestry" : "Back"}</button>
  <div class="crumbs">
    {#each crumbs as crumb, i (i)}
      {#if i > 0}<ChevronRight size={16} />{/if}
      {@const last = i === crumbs.length - 1}
      <button class="crumb" class:last disabled={!crumb.go || last} onclick={() => crumb.go?.()}>
        <crumb.icon size={15} />
        <span class:display={last}>{crumb.label}</span>
      </button>
    {/each}
  </div>
  <span class="note">{note}</span>
  {#if ui.editorActions}
    <button onclick={ui.editorActions.discard} disabled={!ui.editorActions.dirty}>Discard</button>
    <button class="save" onclick={ui.editorActions.save} disabled={!ui.editorActions.dirty}>
      <Check size={14} />{ui.editorActions.saveLabel}
    </button>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0 1rem 0 1.4rem;
    min-height: 3.1rem;
    flex: none;
    background: var(--gold);
    color: #281d1b;
  }
  .bar button {
    border-color: #281d1b;
    padding: 0.38rem 0.7rem;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    overflow: hidden;
  }
  .crumb {
    border: none !important;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.9rem;
    font-weight: 500;
    padding: 0.2rem 0.3rem !important;
    min-width: 0;
  }
  .crumb:disabled {
    opacity: 1;
  }
  .crumb span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crumb.last span {
    font-size: 1.45rem;
    font-weight: 400;
  }
  .note {
    margin-left: auto;
    font-size: 0.78rem;
    opacity: 0.75;
    white-space: nowrap;
  }
  .save {
    background: var(--accent);
    border-color: var(--accent) !important;
    color: var(--accent-text);
  }
</style>
