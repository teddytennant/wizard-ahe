FROM wizard-ahe/task-base:latest

WORKDIR /app

COPY <<'EOF' /app/src/01_header.txt
== HEADER ==
EOF

COPY <<'EOF' /app/src/02_body.txt
body line one
body line two
EOF

COPY <<'EOF' /app/src/10_footer.txt
== FOOTER ==
EOF
