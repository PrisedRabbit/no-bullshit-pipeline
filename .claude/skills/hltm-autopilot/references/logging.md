# Logging

## Autopilot Log

Location: `{PROJECT_ROOT}/.hltm/autopilot.log`

## Format

```
YYYY-MM-DDTHH:MM:SSZ [LEVEL] phase=<phase> message
```

## Levels

- `INFO`: Normal operations
- `WARN`: Recoverable issues
- `ERROR`: Errors requiring user action
- `DEBUG`: Detailed execution info

## Examples

```
2026-02-04T10:15:30Z [INFO] phase=idle message="Waiting for input/brief.md"
2026-02-04T10:20:00Z [INFO] phase=brief message="Greenfield detected, selecting blueprint"
2026-02-04T10:25:00Z [INFO] phase=planning message="Creating PRD"
2026-02-04T11:00:00Z [WARN] phase=solutioning message="Questions created in input/questions.md"
2026-02-04T11:05:00Z [ERROR] phase=implementation story=1-2-refresh-token message="Max attempts reached, blocked=true"
2026-02-04T12:00:00Z [INFO] phase=retro message="Cleanup completed, changelog updated"
```

## Implementation

Autopilot should:
1. Create/append to `.hltm/autopilot.log`
2. Log each phase transition
3. Log errors and warnings
4. Keep stdout for user-facing messages

## Log Rotation

Manual rotation recommended:
```bash
# Archive old logs
mv .hltm/autopilot.log .hltm/autopilot-$(date +%Y%m%d).log

# Or truncate if too large
tail -n 1000 .hltm/autopilot.log > .hltm/autopilot.log.tmp
mv .hltm/autopilot.log.tmp .hltm/autopilot.log
```
