---
description: AsterYggdrasil docs contributing guide covering placement, reader tasks, terminology, links, and validation.
---

# Docs Contributing

This page is for people editing the AsterYggdrasil docs. Every page should help the reader complete a clear task: run the service, create a profile, connect a launcher, configure public URLs, diagnose textures, back up data, or understand project boundaries.

## Decide Where It Belongs

| What are you writing? | Put it in | Examples |
| --- | --- | --- |
| First run, player actions, launcher setup | `guide/` | Getting Started, User Guide, Launcher Login |
| Yggdrasil protocol and texture behavior | `guide/` | Yggdrasil API, Textures, Minecraft Profiles |
| Admin config and runtime policy | `guide/` | Config and Keys, Texture Storage, Audit and Tasks |
| Deployment, reverse proxy, backups, launch checks | `deployment/` | Docker Deployment |
| Project boundaries, contribution rules, routing help | `guide/` reference pages | About, Docs Contributing |

When unsure, ask: **what task is the reader trying to complete?**

- "I need to run it first" -> `guide/getting-started`
- "I want to use this feature" -> `guide/`
- "I need launcher or server integration" -> `guide/launcher-login` or `guide/yggdrasil-api`
- "I need production configuration" -> `guide/configuration` or `deployment/`
- "I do not understand the project boundary" -> `guide/about`

## Be Careful With Top Navigation

The top navigation is for broad paths only:

- Home
- Getting Started
- Guides
- Integration
- Deployment
- About
- Version

Add new pages to the fixed sidebar first. Change the top navigation only when there is a new first-level reader task.

## The Sidebar Is a Reading Flow

The sidebar is fixed for the whole docs site and is not sorted by filename. Default order:

1. Start
2. Player Usage
3. Protocol Integration
4. Admin Maintenance
5. Deployment
6. Project Reference

Insert new pages where readers first need them.

## Keep Terminology Domain-Specific

Prefer product and UI terms. Add English or internal names on first use only when helpful.

Recommended terms:

- `site account`
- `Minecraft profile`
- `profile name`
- `wardrobe`
- `skin`
- `cape`
- `texture`
- `Yggdrasil API`
- `authlib-injector`
- `public site URL`
- `skinDomains`
- `signing key`
- `audit log`

Do not bring in old template or unrelated file-drive concepts such as file sharing, teams, trash, or cloud drive. When documenting stored skin/cape files, explain them as Minecraft textures, not as generic file-manager objects.

## Orient the Reader First

Long pages should start with:

- what the page covers
- when to read it
- where the reader should operate next

Suggested structure:

```md
# Page Title

::: tip What this covers
One sentence describing the boundary. Avoid repeating neighboring pages.
:::

## Quick Routing

| What you want to do | Where to go |
| --- | --- |
| ... | ... |
```

## Link Rules

Prefer absolute internal links:

```md
[Config and Keys](/en/guide/configuration)
[Yggdrasil API](/en/guide/yggdrasil-api)
[Docker Deployment](/en/deployment/docker)
```

Same-directory links are acceptable, but avoid complex `../guide/...` paths across directories. Absolute links are easier to read and safer when files move.

## Writing Rules

- Give the conclusion before details.
- Use tables for quick lookup and lists for steps.
- Put config keys, paths, commands, and API paths in backticks.
- Use `warning` for risky operations.
- Use `details` for optional background.
- Do not promise features that are not merged.
- Link to related pages instead of duplicating large blocks.
- Keep Yggdrasil protocol APIs separate from `/api/v1` site APIs; they use different response formats.

## Validate Changes

After editing docs, run at least:

```bash
cd docs
bun run docs:build
```

If you changed navigation, sidebar, or the homepage, also run:

```bash
cd docs
bun run docs:dev
```

Then click through:

- homepage entries
- top navigation
- fixed sidebar
- new pages
- matching Chinese/English pages
- code blocks and tables in light/dark mode

Building is only the baseline. The real check is whether users can find the right page from the entry points.
