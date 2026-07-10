Kill a running background task started with execute run_in_background.

Note: only use this for agent-managed jobs. Do not start long-lived services (HTTP, QEMU, daemons that verifiers need after the session) via run_in_background — use `nohup ... &` instead so they are not tied to the agent task registry.
