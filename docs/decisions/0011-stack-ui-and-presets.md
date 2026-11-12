# 0011. Saved stack, presets, drag-and-drop stack UI

- Status: Accepted
- Date: 2026-11-12
- Amends: [0010](0010-scene-stack.md)

## Context

After 0010, the stack's behaviour was defined, but not how it's used at the table or what
persists between sessions. Feedback on 0010:

- Each session should start from the last stack.
- Common stacks should be reusable.
- The stack should be the focus of the UI, driven by drag and drop, including inserting between scenes.
- Scenes should be disableable without being removed.
- Ducking should be stronger than "lower by 30%".

## Decision

**Persistence**

- The current stack is saved in `comilito.json` (`stack`, ordered bottom to top) and restored on
  open, **paused** until the user presses play.
- **Stack presets** (`stackPresets`) are named, saved stacks. They can be loaded (replacing the
  stack) or dragged into the stack (inserting their scenes at the drop position).

**Stack entries**

- Each entry has its own `id`, a `scene` reference and an `enabled` flag.
- Disabled entries keep their position and are ignored by the mix calculation. Their music
  pauses and resumes when re-enabled.

**Ducking** (changes 0010)

- `duck` (lower *by*) is replaced by `duckLevel` (lower *to*). The default is **0.3**: layers
  below play at 30% of their volume. It multiplies across levels (0.3 × 0.3 = 9%).

**Controls** (replaces the controls in 0010)

- Drag scenes from the library into the stack at any position. Drag to reorder, and drag out or
  press × to remove. Use a checkbox to enable or disable.
- Right-click menu: *Add on top*, *Switch stack to this scene*, and *Replace stack* (presets).
- Scene hotkey: adds the scene on top if it isn't in the stack; otherwise toggles its enabled state.
- **Space**: pause everything / resume. **Esc**: stop all (fade out, reset positions, keep the stack).
- **Duplicates:** adding a scene already in the stack asks *Move it here / Add a duplicate / Cancel*.

**Layout:** a library (presets, then scenes) on the left, the stack in the centre (top of the stack
at the top), the effects pad on the right, and the mixer bar at the bottom.

Details: [../scenes.md](../scenes.md).

## Consequences

- The stack changes often, so `comilito.json` writes are debounced (for example, 1 s after the
  last change) and remain atomic (0006).
- Presets refer to scenes by id. Deleting a scene must also remove it from presets and the
  saved stack (the UI warns and lists them).
- Drag and drop needs a frontend library or careful hand-written code. That choice is made when
  the UI is built, following the dependency rule in CLAUDE.md.
