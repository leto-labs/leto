# Agent Browser

## Summary

Vercel's `agent-browser` is the strongest newly found local CLI contender
beyond `@playwright/cli` and `browser-use`.

The project is unusually direct about its purpose: a headless browser
automation CLI for AI agents, implemented as a native Rust binary.

For this repo, that matters because the tool surface is broad enough to use for
real development/debugging work, not just toy demos.

## Confirmed Capabilities

From the README:

- installable globally with `npm install -g agent-browser`, Homebrew, or Cargo
- native Rust CLI with a Chrome-for-Testing install step
- official agent skill install path via `npx skills add vercel-labs/agent-browser`
- direct commands for:
  - `open`, `click`, `type`, `fill`, `hover`, `drag`, `upload`
  - `snapshot` and annotated `screenshot`
  - tabs, windows, frames, dialogs, clipboard, mouse, and keyboard control
  - cookies, localStorage, and sessionStorage inspection/mutation
  - network inspection, request filtering, routing, and HAR recording
  - console/error inspection
  - trace recording, profiling, visual diffing, and snapshot diffing
- explicit session and persistence support:
  - named sessions
  - persistent profiles
  - auth state save/load
  - auto-connect/import from an existing Chrome session
- user-level config at `~/.agent-browser/config.json`
- project-level config at `./agent-browser.json`
- runtime WebSocket streaming via `agent-browser stream enable|status|disable`

That is a deeper operational surface than most agent-oriented browser CLIs.

## Why It Matters For This Repo

If we want Codex to drive OpenCode during local development, this CLI maps
cleanly onto the actual debugging needs:

- open the local UI
- inspect current state through snapshots
- click and type through the app
- read console and network failures
- save or restore browser/auth state
- diff before/after page behavior when a server change lands

The persistence model is especially useful for the OpenCode workflow because
our first-run attach requires a compat-path server entry. A CLI that can keep
browser state, localStorage, and auth across runs lowers friction a lot.

The official skill matters too. This is not just "a CLI we could maybe teach to
the agent." Upstream already ships a maintained `SKILL.md` intended for coding
assistants, including Codex.

## Setup Recommendations

For this repo, the most practical setup looks like:

1. Install the CLI and bundled browser path:
   - `npm install -g agent-browser`
   - `agent-browser install`
2. Install the official skill instead of copying ad hoc prompts:
   - upstream generic install: `npx skills add vercel-labs/agent-browser`
   - in this repo, prefer the standard layout command from `AGENTS.md`:
     `skills add vercel-labs/agent-browser --agent claude-code cursor -y`
3. Use headed mode while debugging local OpenCode sessions:
   - `agent-browser --headed open http://127.0.0.1:3000`
4. Use a dedicated persistent profile for this repo or this workflow:
   - `agent-browser --profile ~/.agent-browser/profiles/mauser-opencode --headed open http://127.0.0.1:3000`
5. If you want to inspect runtime streaming state, enable the built-in stream
   endpoint for the session:
   - `agent-browser stream enable`
   - `agent-browser stream status`

The main workflow advantage is that a dedicated profile can preserve the
OpenCode localStorage/server selection state between runs, which directly
reduces the friction we saw when attaching the UI to
`/v1/compat/opencode`.

## Version Notes

This repo is pinned to `agent-browser v0.22.3` because that matches the local
CLI installation used during research.

One important consequence: the installed/pinned version does **not** expose the
newer `dashboard` subcommand. The relevant observability surface in `v0.22.3`
is session streaming:

- `agent-browser stream enable`
- `agent-browser stream status`
- `agent-browser stream disable`

If later research turns up `dashboard` in newer upstream docs or branches,
those commands should not be treated as available in the pinned `v0.22.3`
source snapshot without upgrading the local CLI and this research note.

## Environment Notes

On Ubuntu 24.04-style setups with AppArmor-restricted user namespaces, the
bundled Chrome may fail to launch with Chromium's "No usable sandbox" error.

In that environment, `agent-browser` needs an explicit launch arg workaround:

```bash
agent-browser --args '--no-sandbox' open http://127.0.0.1:3000
```

That is an environment-specific workaround, not a general recommendation for
all machines.

## Browser Choice And Brave

The safest default is still to use the bundled Chrome-for-Testing path for
automation, even if Brave is your daily browser.

Reasons:

- it is the browser path upstream documents most directly
- it avoids interference from personal extensions and day-to-day browsing state
- it keeps automation runs more reproducible

If you mainly use Brave, there are still two good ways to use it:

### 1. Brave as the automation browser

Use Brave through a dedicated automation profile and an explicit executable
path:

```bash
agent-browser \
  --executable-path /usr/bin/brave-browser \
  --profile ~/.agent-browser/profiles/mauser-opencode-brave \
  --headed \
  open http://127.0.0.1:3000
```

This is the cleanest Brave-based setup when you want the agent to drive Brave
directly.

### 2. Brave as the auth source

If you are already logged into something in Brave and want to import that auth,
launch Brave with remote debugging and connect explicitly:

```bash
brave-browser --remote-debugging-port=9222
agent-browser --cdp 9222 state save ./auth.json
```

Then use the saved state in an isolated automation session.

### Recommendation

For repeatable repo development, I would not use your daily Brave profile
directly. Either:

- use bundled Chrome-for-Testing for most automation, or
- use Brave with a dedicated automation profile

I would only attach to your everyday Brave profile as a temporary auth-import
step.

## Dedicated Profile Recommendation

Yes, I recommend creating a dedicated browser profile.

That is the right default because it separates:

- daily browsing state
- development automation state
- repo-specific localStorage/auth/server configuration

For this repo, a good pattern would be one dedicated profile such as:

- `~/.agent-browser/profiles/mauser-opencode`

or, if you want to be explicit about Brave:

- `~/.agent-browser/profiles/mauser-opencode-brave`

That keeps:

- OpenCode default server settings
- auth cookies
- IndexedDB and service workers
- any UI-specific state we set during debugging

isolated from your normal browsing environment.

## Tradeoffs

- It looks newer and less battle-tested than Playwright itself.
- It is a direct command surface, not a higher-level local SDK with
  deterministic workflow caching.
- The feature set is wide, but the ecosystem and external examples are still
  thinner than Playwright's.

## Working Recommendation

Treat `agent-browser` as the strongest "new entrant" in this research set.

If `@playwright/cli` feels too tied to the Playwright ecosystem, this is the
best next CLI to try for serious local browser debugging.

It is especially compelling if we value:

- native CLI ergonomics
- strong session/state handling
- built-in debugging artifacts like HAR, traces, console logs, and diffs
- an official upstream-maintained skill rather than a repo-local wrapper
- a version-pinned repocache source tree that matches the installed CLI

## Key Sources

- README:
  <https://raw.githubusercontent.com/vercel-labs/agent-browser/main/README.md>
- Official skill:
  <https://raw.githubusercontent.com/vercel-labs/agent-browser/main/skills/agent-browser/SKILL.md>
