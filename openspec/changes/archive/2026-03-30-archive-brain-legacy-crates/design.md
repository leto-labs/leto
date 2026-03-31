# Design: archive-brain-legacy-crates

## Decision: Keep Legacy Source Local But Untracked

Legacy `brain-*` source is still useful for migration-reference work, but it
should not keep occupying committed workspace state.

The chosen mechanism is:

- store the legacy tree under `archive/brain/`
- ignore that subtree in Git
- remove the original tracked paths from the committed tree

This preserves local reference access without implying that the archived crates
are still live product code.

## Decision: Mixed-Use Tooling Must Fail Clearly

Files that still serve current workflows should stay committed, but their
legacy `brain` / `brain-acp` branches should not silently fall through.

When a user asks for an archived Harbor surface, the committed runner should
fail with an explicit archive message rather than pretending the legacy agent is
still supported.

## Decision: Canonical OpenSpec Must Track The Committed Tree

Once the legacy crates leave the committed workspace, canonical OpenSpec should
stop describing those capabilities as current truth.

Historical `brain-*` specs can remain in a committed archive tree, but they
must not stay under `openspec/specs/`.
