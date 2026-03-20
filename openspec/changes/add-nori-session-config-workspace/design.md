# Design: add-nori-session-config-workspace

## Decision: Develop Against A Real Nori Fork Workspace

The repository will carry a checked-out Nori fork at `submodules/nori-cli`.

This is the local development workspace for the Nori work. Contributors can
make and test the ACP client changes in place while still keeping the actual
branch history in the fork remote.

The first branch reserved for this effort is:

- `feat/add-session-config`

The submodule is intentionally a workspace and integration aid. This change
does not vend Nori as part of the Rust workspace and does not change how the
main project builds.

## Decision: Scope The First Client Work To Session Config Options

ACP session config options are the preferred first target.

The protocol and current reference implementations indicate that richer
session-level controls should flow through `configOptions` rather than through
session modes or model-specific hacks.

That means the first Nori fork branch should target:

- ACP `configOptions`
- `session/set_config_option`
- `ConfigOptionUpdate` refresh handling

The first branch should not implement ACP session modes.

Modes remain a follow-up question only if interop or user experience later
demands them.

## Decision: Keep The First UI Generic And Additive

The initial Nori change should behave like a generic ACP client, not like a
`brain`-specific frontend.

The client should render whatever session config options the ACP agent exposes
for the current session, subject to a narrow v1 type cut:

- support generic select-style config options
- use the option metadata supplied by the agent
- avoid hardcoding `brain` option IDs or labels

The first UI surface should be a dedicated session-settings command rather than
an extension of existing Nori commands.

This keeps the patch upstream-friendly because it:

- does not overload `/model`
- does not conflict with Nori's existing `/config`
- gives ACP session settings a protocol-aligned home

The recommended initial command name is:

- `/session-settings`

## Decision: Defer Broad Type Coverage

The first Nori implementation should not attempt every theoretical ACP config
shape.

The scoped v1 behavior is:

- render select-style session config options
- submit updates through `session/set_config_option`
- refresh from full option-state updates returned by the ACP session

Unsupported option types, if encountered, should be left for later design
rather than forcing a broad abstraction into the first branch.

## Out Of Scope

This OpenSpec change does not implement:

- generic ACP config-option support in Nori
- ACP session modes in Nori
- any `brain-acp` bridge for reasoning, speed, or other session controls
- changes to `/model`
- changes to `/config`
