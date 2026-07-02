Git is installed in this environment. Create a git repository at `/app/repo`
with exactly this history (three commits on a single branch, no merges), oldest
first:

1. Commit message `step 1` — adds `file1.txt` containing exactly `1` followed
   by a newline.
2. Commit message `step 2` — adds `file2.txt` containing exactly `2` followed
   by a newline.
3. Commit message `step 3` — adds `file3.txt` containing exactly `3` followed
   by a newline.

Requirements:

- Each commit message is exactly the single line shown (`step 1`, `step 2`,
  `step 3`).
- Each commit adds only its own file: the tree at the first commit contains
  only `file1.txt`; at the second, `file1.txt` and `file2.txt`; at the third,
  all three files. No other files may be committed.
- Every commit's author AND committer must be `Eval Bot <eval@example.com>`
  (e.g. set `user.name` and `user.email` in the repo before committing).
- When you are done, the working tree must be clean (`git status --porcelain`
  prints nothing) with all three files present on disk.
