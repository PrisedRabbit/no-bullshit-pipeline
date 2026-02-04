# Errors

| Code     | Message                 | Action                                            |
| -------- | ----------------------- | ------------------------------------------------- |
| **E001** | State corrupted         | Auto-reset to idle. Check git if unexpected.      |
| **E002** | Unknown phase           | Edit session.yaml, fix phase, or reset to idle.   |
| **E003** | Invalid transition      | Reset to last valid phase. Report bug.            |
| **E004** | Max attempts + recovery | See README Troubleshooting.                       |
| **E005** | Sprint-status missing   | Check epics exist, run /bmad-bmm-sprint-planning. |
| **E006** | Brownfield no brief     | Create input/brief.md.                            |
| **W001** | Questions need answers  | Answer input/questions.md, remove waiting flag.   |
| **W002** | Readiness failed        | Auto-returns to solutioning.                      |

Debug: `.hltm/autopilot.log`, `.hltm/session.yaml`, `git log`
