#!/bin/bash
# session-start.sh — SessionStart hook
# Reminds Claude that bpmux is connected.

echo "bpmux is connected. Call set_name first with your role (e.g. 'dev', 'pm', 'designer') so the team knows who joined. Status is managed automatically by hooks — do not call set_status manually. Messages are pushed to you via the bridge — do not poll read_messages."

exit 0
