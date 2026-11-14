# 0031. A scene can duck all the ambience below

- Status: Accepted
- Date: 2026-11-14

## Context

Since [0029](0029-every-layer-has-a-role.md) a scene only ducks the layers below whose role it
has a layer for. A scene with no ambience (music only, or a silence) therefore ducks nothing,
but sometimes it should: battle music on top of a forest should push the birds back.

## Options considered

- **Give such scenes a dummy foreground layer:** works today, but it's a trick, and it only
  ducks foreground layers.
- **Put "No role" back for these cases:** brings back the two rules 0029 removed.
- **A scene switch, "Duck all the ambience below, whatever its role":** one explicit option,
  like *Silence the music of the scenes below*.

## Decision

- A scene may have `ducksAll: true`. While it's enabled, it ducks every ambience layer below it
  to its `duckLevel`, whatever the role. Only written when true; no format version change.
- The switch is in the scene editor's Ambience section, for any scene; it matters most for
  scenes without ambience of their own.

## Consequences

- The ducking rule reads: a scene above ducks a layer if it ducks everything, or if it has a
  layer in the same role.
- The Tapestry says "ducks all ambience below to 30%" on such a scene, and no longer claims a
  scene without ambience ducks anything.
