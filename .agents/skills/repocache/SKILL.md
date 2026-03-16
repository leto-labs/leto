---
name: repocache
description: Research external libraries/frameworks by querying locally cloned repos. Two workflows — (1) configure and clone resources, (2) search cloned source. Use when the user asks about external library APIs, patterns, or usage that may differ from training data. Triggers on "how does X work in <library>", "check the <library> source", "add repocache for <package>", "repocache", when referencing a GitHub URL for research, or when needing up-to-date source-first answers about configured resources. No external CLI dependencies — uses only git and built-in agent tools. Works with any agent (Claude Code, Cursor, OpenCode).
---

# repocache

Query locally cloned repositories for up-to-date, source-first answers about external libraries. Resources use npm package names as identifiers for easy mapping from `package.json` dependencies.

```
repocache/
├── repocache.json      (resource config — tracked)
├── clone.sh            (sync script — tracked)
└── <org>/<repo>/       (shallow clones by git org/repo — gitignored, deduplicated)
```

Multiple npm packages from the same git repo (e.g., `@trpc/server` and `@trpc/client` both from `trpc/trpc`) share a single clone. The `searchPath` field narrows focus to the relevant package within the monorepo.

---

## Part 1: Configuration & Cloning

Two entry points depending on what the user provides. Both follow the same pattern: **get a URL, clone, inspect, configure**.

### Workflow A: GitHub URL

User provides a URL like `https://github.com/org/repo` or `git@github.com:org/repo.git`.

1. **Clone** the repo immediately:
   ```bash
   git clone --depth 1 <url> repocache/<org>/<repo>
   ```
   For HTTPS URLs, the clone script automatically retries with SSH if HTTPS fails.

2. **Discover package name** — inspect the clone:
   ```bash
   # Single-package repo
   cat repocache/<org>/<repo>/package.json   # read "name" field
   # Monorepo
   ls repocache/<org>/<repo>/packages/
   cat repocache/<org>/<repo>/packages/*/package.json
   ```
   - Single-package repo → use `name` from root `package.json`
   - Monorepo → prompt the user which package(s) are relevant, or infer from the project's `package.json` dependencies
   - Not an npm package (tool, docs site, reference) → use the repo name (e.g., `better-context`)

3. **Find a release tag**:
   ```bash
   git ls-remote --tags <url> | sort -V | tail -5
   ```

4. **Find searchPath** — inspect the directory structure:
   - Monorepo: set to the relevant package dir (e.g., `packages/server`)
   - Single-package: set to `src` if it exists

5. **Add entry** to `repocache/repocache.json` with all discovered info and `notes`.

### Workflow B: npm Package Name

User references a package from `package.json` or by npm name.

1. **Resolve git URL**:
   ```bash
   npm view <package-name> repository.url
   # Strip git+ prefix and .git suffix
   ```
   If `npm view` fails or returns no repository, use web search to find the GitHub URL.
   For private repos, use the SSH URL (`git@github.com:org/repo.git`) directly — or use HTTPS and let the clone script fall back to SSH automatically.

2. **Find a release tag** before cloning:
   ```bash
   npm view <package-name> version
   # e.g., 4.7.0 → look for tag v4.7.0
   git ls-remote --tags <url> | grep "v4.7"
   ```
   - Stable release → use matching tag (e.g., `v4.7.0`)
   - Beta/canary release → use `beta` or `canary` branch
   - No suitable tag → use `main`

3. **Add a minimal entry** to `repocache/repocache.json` (name + url + tag):
   ```json
   {
     "name": "@trpc/server",
     "url": "https://github.com/trpc/trpc",
     "tag": "v11.9.0"
   }
   ```

4. **Clone**: `cd repocache && ./clone.sh @trpc/server`

5. **Inspect the clone** to discover searchPath and notes:
   ```bash
   ls repocache/trpc/trpc/packages/
   ```

6. **Refine entry** — update with `searchPath` and `notes`:
   ```json
   {
     "name": "@trpc/server",
     "url": "https://github.com/trpc/trpc",
     "tag": "v11.9.0",
     "searchPath": "packages/server",
     "notes": "Monorepo. Also contains @trpc/client (packages/client) and @trpc/react-query."
   }
   ```

### Refining an Existing Resource

When using a resource, improve its config for future searches:

- **Pin `tag`** — if still on `main`, find a release tag matching the project's dependency version.
- **Set `searchPath`** — if searching a monorepo and hitting irrelevant packages, narrow to the right subdirectory.
- **Add `notes`** — record what you learned about the repo layout (monorepo structure, related packages, important directories).
- **Tune `include`/`exclude`** — if default patterns return too much noise or miss important files.

### Config Schema

| Field        | Required | Default       | Description                                             |
| ------------ | -------- | ------------- | ------------------------------------------------------- |
| `name`       | Yes      |               | npm package name, or repo name if not an npm package    |
| `url`        | Yes      |               | Git repo URL (HTTPS or SSH). HTTPS URLs automatically fall back to SSH if cloning fails (e.g., private repos). |
| `tag`        | No       | `"main"`      | Git tag (preferred) or branch to clone                  |
| `searchPath` | No       |               | Subdirectory to focus searches within                   |
| `include`    | No       | from defaults | Glob patterns for relevant files                        |
| `exclude`    | No       | from defaults | Glob patterns to skip                                   |
| `notes`      | No       |               | Hints about repo structure for the agent                |
| `enabled`    | No       | `false`       | Set to `true` to clone/update this resource             |

### Clone Script

```bash
cd repocache && ./clone.sh              # clone/pull all resources
cd repocache && ./clone.sh <name>       # clone/pull one resource by npm name
cd repocache && ./clone.sh --no-update  # clone missing only, skip pulls
```

- New repos: shallow clone (`--depth 1`) for disk efficiency
- Existing repos: `git fetch` + checkout to update
- Deduplicates by URL within a single run (monorepo packages sharing a repo)
- **SSH support**: SSH URLs (e.g., `git@github.com:org/repo.git`) are used directly. HTTPS URLs automatically fall back to SSH if the clone fails (useful for private repos requiring SSH keys).

---

## Part 2: Searching

When answering questions about a library, search within the cloned source.

### Search Process

1. **Find resource** — Read `repocache/repocache.json`, match by `name`.
2. **Resolve disk path** — Extract `org/repo` from `url`. Clone lives at `repocache/<org>/<repo>/`.
3. **Ensure cloned** — If the directory doesn't exist, run `./clone.sh <name>`.
4. **Build search root** — Combine disk path + `searchPath`:
   ```
   repocache/<org>/<repo>/<searchPath>/
   ```
5. **Search** using available tools (Grep, Glob, SemanticSearch, Read, etc.):
   - Grep for symbols, function names, patterns
   - Glob to find files by name
   - Read for specific files once located
   - Respect `include`/`exclude` patterns to filter results
6. **Check notes** — the `notes` field often contains important context about repo layout.

### Subagent Strategy (when Task tool is available)

If the agent supports spawning subagents (e.g., Claude Code Task tool, OpenCode), prefer delegating repocache searches to a subagent. This keeps the main context clean and allows parallel research.

Spawn the subagent with:
- **Working directory**: `repocache/<org>/<repo>/<searchPath>/`
- **Prompt**: the specific question about the library
- **Context**: include the resource's `notes` field and any relevant `include`/`exclude` patterns

When subagents are not available (e.g., Cursor), search directly in the main context using the same process above.

### Search Tips

- Start broad with Grep on the search root, then narrow to specific files.
- For large repos, Glob first to understand file structure before reading.
- For monorepos without `searchPath`, check `packages/` or `src/` first.
- After a successful search, refine the resource config (Part 1) to make future searches faster.
