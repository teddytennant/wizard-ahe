Return the status and buffered output (stdout+stderr tail) of a background task started with execute run_in_background.

For services that must outlive the agent (web servers, VMs), prefer `nohup ... &` and check logs with ordinary shell commands rather than this tool.
