#!/bin/bash
# Verifier for the makefile-build task. Drives make build/test/clean through a
# full lifecycle, including a corrupted bundle and a newly added source file.
mkdir -p /logs/verifier

fail() {
  echo "[verifier] FAIL: $1"
  echo "0" > /logs/verifier/reward.txt
  echo "[verifier] makefile-build -> reward=0"
  # Restore the pristine src/ for idempotent re-runs.
  rm -f /app/src/00_alpha.txt
  exit 0
}

cd /app || fail "cannot cd /app"
[ -f Makefile ] || fail "/app/Makefile missing"

# Idempotency: drop the extra file a previous verifier run may have left.
rm -f src/00_alpha.txt

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/expected1" <<'EOF'
== HEADER ==
body line one
body line two
== FOOTER ==
EOF
cat > "$work/expected2" <<'EOF'
alpha
== HEADER ==
body line one
body line two
== FOOTER ==
EOF

# clean succeeds even when dist is absent, and is idempotent.
make clean >/dev/null 2>&1 || fail "make clean failed"
make clean >/dev/null 2>&1 || fail "make clean not idempotent"
[ ! -e dist ] || fail "make clean left dist behind"

# test fails when the bundle is missing.
if make test >/dev/null 2>&1; then fail "make test passed with dist/bundle.txt missing"; fi

# Plain make runs build (default target).
make >/dev/null 2>&1 || fail "plain 'make' failed"
[ -f dist/bundle.txt ] || fail "'make' did not create dist/bundle.txt"
cmp -s dist/bundle.txt "$work/expected1" || fail "bundle content wrong after 'make'"

make test >/dev/null 2>&1 || fail "make test failed on a correct bundle"

# test detects a corrupted bundle; build repairs it.
echo "corrupted" >> dist/bundle.txt
if make test >/dev/null 2>&1; then fail "make test passed on a corrupted bundle"; fi
make build >/dev/null 2>&1 || fail "make build failed"
cmp -s dist/bundle.txt "$work/expected1" || fail "make build did not repair the bundle"
make test >/dev/null 2>&1 || fail "make test failed after rebuild"

# build must pick up newly added source files (no hard-coded file list).
printf 'alpha\n' > src/00_alpha.txt
if make test >/dev/null 2>&1; then fail "make test passed on a stale bundle after adding src/00_alpha.txt"; fi
make build >/dev/null 2>&1 || fail "make build failed with a new source file"
cmp -s dist/bundle.txt "$work/expected2" || fail "bundle wrong after adding src/00_alpha.txt (must be sorted, glob-based)"
make test >/dev/null 2>&1 || fail "make test failed after rebuilding with the new file"

# clean removes dist and leaves src alone.
make clean >/dev/null 2>&1 || fail "final make clean failed"
[ ! -e dist ] || fail "final make clean left dist behind"
for f in src/00_alpha.txt src/01_header.txt src/02_body.txt src/10_footer.txt; do
  [ -f "$f" ] || fail "make clean removed $f"
done

rm -f src/00_alpha.txt

echo "1" > /logs/verifier/reward.txt
echo "[verifier] makefile-build -> reward=1"
