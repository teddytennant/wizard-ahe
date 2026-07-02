# Wizard harness bundle

This directory is a *harness bundle*: the evolvable surface of the wizard
agent, externalized as plain files. Wizard loads it when started with
`--harness-dir <this dir>` (or `$WIZARD_HARNESS_DIR`); every component
present here shadows the corresponding compiled default, and a missing or
empty file falls back to that default.

## Components

- `system_prompt.md` — the base personality prompt (sovereign mode). The
  wizard charter, skills index, project instructions, and memory sections
  are appended on top at runtime and cannot be edited from here.
- `tool_descriptions/<tool>.md` — the description advertised to the model
  for the named native tool. Only the description is overridable; tool
  behavior, parameters, and access class are compiled in.
- `skills/<name>/SKILL.md` — skills loaded into the prompt's skills section.
  Bundle skills shadow bundled and user skills by name; new directories add
  new skills.
- `subagents/<name>.toml` — spawnable subagent definitions (`name`,
  `description`, `system_prompt`, optional `tool_scope`, `max_steps`).
  Bundle definitions shadow user-defined and built-in ones by name.

## Editing rules for evolution loops

- Keep names stable: a `tool_descriptions/` file must keep the exact tool
  name as its stem, a subagent TOML must keep its `name` field matching new
  file names you introduce.
- Edits take effect on the next wizard start (or `/reload` in a session);
  no rebuild is required.
- Deleting a file reverts that component to the compiled default, so
  destructive experiments are always recoverable.
