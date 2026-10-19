# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Canonical role    | Label in our tracker | Meaning                                      |
| ----------------- | -------------------- | -------------------------------------------- |
| `ready-for-agent` | `agent-todo`         | Fully specified, ready for an AFK agent      |
| `needs-info`      | `needs-info`         | Waiting on the reporter for more information |
| `needs-triage`    | (not used)           | Leave the issue unlabeled                    |
| `ready-for-human` | (not used)           | Leave the issue unlabeled                    |
| `wontfix`         | (not used)           | Close the issue with a comment, no label     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table. For a role marked "not used", do what the Meaning column says and do not create the label.

Extra label, not a canonical role:

| Label        | Meaning                                                      |
| ------------ | ------------------------------------------------------------ |
| `agent-done` | An agent finished the work; a human reviews it and closes it |

When an agent finishes an `agent-todo` issue, remove `agent-todo` and add `agent-done`.
