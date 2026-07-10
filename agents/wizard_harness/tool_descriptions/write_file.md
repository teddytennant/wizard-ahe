Create or overwrite a file with the given content, creating parent directories as needed.

Tips:
- Use for new files or full rewrites; prefer `edit_file` for surgical changes to existing code.
- Write required deliverables (reports, move files, polyglot sources) as soon as you know the path and a plausible schema-valid payload — do not defer the only required output until a narration-only final turn. Prefer an early correct draft over finishing with a missing file.
- When a verification script already found the answer (e.g. mate UCIs printed in stdout), write the file **now** (or have that script write it) — do not open another analysis loop while the path is still missing.
- For JSONL/CWE reports, copy the demonstration schema exactly: key names, types (`cwe_id` is a **list** of lowercase `cwe-N` strings, not a bare string and not `CWE-N`), and path style from the example. **Only** use IDs that appear in the task's candidate list (e.g. `["cwe-93","cwe-20"]` for listed CRLF/input-validation header bugs). Never write only an unlisted synonym like `cwe-113`.
- When the contract asks for multiple answers (all winning moves, all IDs), write every required line/value in one go (e.g. both mate UCIs, one per line).
