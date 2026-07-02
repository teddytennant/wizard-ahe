#!/bin/bash
# Verifier for the git-history task. Checks commit count, messages, author and
# committer identity, the exact tree at every commit, file contents, and a
# clean working tree.
mkdir -p /logs/verifier

fail() {
  echo "[verifier] FAIL: $1"
  echo "0" > /logs/verifier/reward.txt
  echo "[verifier] git-history -> reward=0"
  exit 0
}

REPO=/app/repo
[ -d "$REPO/.git" ] || fail "$REPO is not a git repository"

# The verifier may run as a different uid than the one that created the repo.
export HOME=/tmp/verifier-home
mkdir -p "$HOME"
git config --global --add safe.directory "$REPO" 2>/dev/null

g() { git -C "$REPO" "$@"; }

count=$(g rev-list --count HEAD 2>/dev/null) || fail "cannot read history (no commits on HEAD?)"
[ "$count" = "3" ] || fail "expected exactly 3 commits, found $count"

# No merge commits.
merges=$(g rev-list --merges --count HEAD)
[ "$merges" = "0" ] || fail "history contains merge commits"

# Messages, author, committer — oldest first.
meta=$(g log --reverse --format='%s|%an|%ae|%cn|%ce')
expected_meta='step 1|Eval Bot|eval@example.com|Eval Bot|eval@example.com
step 2|Eval Bot|eval@example.com|Eval Bot|eval@example.com
step 3|Eval Bot|eval@example.com|Eval Bot|eval@example.com'
if [ "$meta" != "$expected_meta" ]; then
  fail "commit metadata mismatch; got: $meta"
fi

# Exact tree at each commit.
tree1=$(g ls-tree -r --name-only 'HEAD~2')
tree2=$(g ls-tree -r --name-only 'HEAD~1')
tree3=$(g ls-tree -r --name-only 'HEAD')
[ "$tree1" = "file1.txt" ] || fail "tree at 'step 1' is [$tree1], expected only file1.txt"
[ "$tree2" = "$(printf 'file1.txt\nfile2.txt')" ] || fail "tree at 'step 2' is [$tree2], expected file1.txt and file2.txt"
[ "$tree3" = "$(printf 'file1.txt\nfile2.txt\nfile3.txt')" ] || fail "tree at 'step 3' is [$tree3], expected all three files"

# Exact committed file contents (trailing newline required).
for n in 1 2 3; do
  content=$(g show "HEAD:file$n.txt" | od -An -c | tr -d ' \n')
  expected=$(printf '%s\n' "$n" | od -An -c | tr -d ' \n')
  [ "$content" = "$expected" ] || fail "HEAD:file$n.txt content wrong (got od: $content)"
done

# Clean working tree with the files on disk.
status=$(g status --porcelain)
[ -z "$status" ] || fail "working tree not clean: $status"
for n in 1 2 3; do
  [ -f "$REPO/file$n.txt" ] || fail "file$n.txt missing from the working tree"
done

echo "1" > /logs/verifier/reward.txt
echo "[verifier] git-history -> reward=1"
