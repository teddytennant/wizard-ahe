#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
# Note: recipe lines below are indented with literal tabs, as make requires.
set -e
cat > /app/Makefile <<'EOF'
.PHONY: build test clean

build:
	mkdir -p dist
	cat $(sort $(wildcard src/*.txt)) > dist/bundle.txt

test:
	@test -f dist/bundle.txt
	@cat $(sort $(wildcard src/*.txt)) | cmp -s - dist/bundle.txt

clean:
	rm -rf dist
EOF
