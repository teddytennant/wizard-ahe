FROM python:3.12-slim

WORKDIR /app

COPY <<'EOF' /app/logs/app.log
2026-06-01T08:00:12 INFO service starting
2026-06-01T08:00:13 DEBUG loading config from /etc/app.conf
2026-06-01T08:00:14 INFO config loaded
2026-06-01T08:00:20 DEBUG connecting to database
2026-06-01T08:00:21 ERROR database connection refused
2026-06-01T08:00:25 WARNING retrying database connection
2026-06-01T08:00:26 INFO database connected
2026-06-01T08:01:02 INFO listening on port 8080
2026-06-01T08:03:11 DEBUG GET /health
2026-06-01T08:03:11 INFO health check ok
2026-06-01T07:59:58 DEBUG early boot probe
2026-06-01T08:05:40 WARNING slow query took 2.3s
2026-06-01T08:06:02 ERROR timeout talking to payments service
2026-06-01T08:06:03 WARNING falling back to cached rates
2026-06-01T08:07:19 INFO processed order 1001
2026-06-01T08:07:44 INFO processed order 1002
this line is garbage and must be ignored
2026-06-01T08:08:01 ERROR timeout talking to payments service
2026-06-01T08:08:02 DEBUG retry scheduled
2026-06-01T08:09:15 INFO processed order 1003
2026-06-01T08:09:30 WARNING disk usage at 91%
2026-06-01T08:10:05 ERROR timeout talking to payments service
2026-06-01T08:10:06 INFO circuit breaker opened
2026-06-01T08:12:47 DEBUG GET /health
2026-06-01T08:12:47 INFO health check ok
2026-06-01T08:14:09 ERROR database connection refused
2026-06-01T08:15:22 INFO processed order 1004
2026-06-01T08:20:00 INFO shutdown requested
2026-06-01T08:20:01 DEBUG flushing buffers
2026-06-01T08:16:33 WARNING unclosed session detected
2026-06-01T08:20:02 INFO service stopped
EOF
