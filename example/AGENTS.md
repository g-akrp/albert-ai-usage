# Agent Usage providers

Maestri shows plan usage rings for coding agents. Each JSON file in this folder is one provider: which program to run, and how to read its answer. Maestri never reads tokens or calls vendor APIs itself; it only runs what these files say.

Maestri writes this guide and replaces it when it changes, so keep your own notes elsewhere. Turning Agent Usage off in Settings deletes this whole folder, including provider files you added.

## Basics

- One provider per file, named `<id>.json`. The id is 1 to 40 lowercase letters, digits, or hyphens, starts with a letter or digit, and must match the file's `"id"`.
- Maestri rereads this folder every 15 seconds. Settings → Agents → Usage lists every provider, with the exact error for a file that doesn't load (for example `/source/args/2: Expected a string argument`).
- `claude.json`, `codex.json`, and `antigravity.json` ship with Maestri. Editing one marks it Modified, and Restore Default brings the shipped version back. A deleted one stays deleted until restored. To make a variation, copy a file to a new id instead of editing it.
- Only regular files up to 256 KB load, not symlinks. Unknown keys are errors, so a typo never runs as something else.
- Each provider has its own switch and ring color in Settings.

## How a provider runs

- Maestri launches the program directly, without a shell, from an empty temporary folder, in its own process group. Its stderr is discarded.
- The environment is minimal: `PATH` (your login shell's PATH plus common install folders), `HOME`, `TERM=dumb`, `NO_COLOR=1`, and, when Maestri has them, `USER`, `LOGNAME`, `LANG` (`en_US.UTF-8` otherwise), `LC_ALL`, `TMPDIR`, `SHELL`, `SSH_AUTH_SOCK`, and `__CF_USER_TEXT_ENCODING`. Add anything else with `env`.
- Polls run every `refresh.intervalSeconds`, three times less often while Maestri is in the background, and only while the rings are on screen. At most two providers run at once, each provider's requests are at least 60 seconds apart, and failures back off up to 30 minutes.

## File layout

```json
{
  "schemaVersion": 1,
  "id": "my-agent",
  "revision": 1,
  "name": "My Agent",
  "source": { },
  "map": { },
  "refresh": { "intervalSeconds": 300 }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `schemaVersion` | yes | Always `1`. |
| `id` | yes | Same as the file name without `.json`. |
| `revision` | yes | A whole number from 1. Yours to bump; Maestri doesn't use it for your files. |
| `name` | yes | Shown in Maestri, 1 to 64 characters. |
| `source` | yes | How to get usage. See Sources. |
| `map` | yes | How to read it. See Map. |
| `refresh.intervalSeconds` | no | 60 to 86400. Default 300. |

## Sources

Both source types share these keys:

| Key | Meaning |
| --- | --- |
| `type` | `"command"` or `"stdio"`. |
| `executable` | A program name found on PATH, such as `"codex"` (1 to 64 letters, digits, `.`, `_`, `-`, or `+`), or a path starting with `/` or `~/`. |
| `args` | Up to 32 strings, passed as is. No shell expansion, no `~`. |
| `env` | Up to 16 variables set on top of the minimal environment. Names use letters, digits, and underscores and don't start with a digit. Values are strings up to 1024 bytes; a leading `~/` means your home folder. |
| `timeoutSeconds` | Covers the whole run. |

### `command`

Runs the program with stdin at end of input (`/dev/null`) and reads stdout as one JSON document. A nonzero exit fails the run.

| Key | Meaning |
| --- | --- |
| `timeoutSeconds` | 1 to 120. Default 30. |
| `maxOutputBytes` | 1024 to 8388608. Default 1048576. |
| `expect` | Optional checks on the document: `match`, `error`, `require` (below). |

### `stdio`

Holds a short exchange of newline-delimited JSON: write a message, wait for its answer, repeat. Lines that don't match an `await` are skipped, including lines that aren't JSON. When the steps finish, stdin is closed.

| Key | Meaning |
| --- | --- |
| `steps` | 1 to 16 steps, each exactly `{ "write": { ... } }` (a JSON object up to 64 KB, sent as one line) or `{ "await": { ... } }`. |
| `output` | The `capture` name of the message the map reads. |
| `timeoutSeconds` | 1 to 60. Default 15. |
| `maxLineBytes` | 1024 to 8388608. Default 1048576. |
| `maxTotalBytes` | `maxLineBytes` to 16777216. Default 4194304, or `maxLineBytes` if that's larger. |

An `await` takes:

| Key | Meaning |
| --- | --- |
| `match` | 1 to 8 predicates. The first message where all hold is the answer. |
| `error` | A pointer. If it's present and not null in the answer, the run fails with its text (or its `message` member). A JSON-RPC `code` of -32601 reads as "Update the CLI to see usage". |
| `require` | A pointer that must be present and not null in the answer. |
| `capture` | A name for the answer: a letter, then up to 31 letters, digits, or underscores. |

`expect` on a command takes the same `match` (0 to 8 predicates), `error`, and `require`, without `capture`.

A predicate is `{ "path": "/type", "equals": "control_response" }` (a string, number, boolean, or null; `2` equals `2.0` but not `"2"`) or `{ "path": "/result", "exists": true }`.

## Map

Paths are JSON Pointers such as `/rate_limits/five_hour`, relative to the value in scope: `root` for the top-level keys, each meter's value for its keys, each window's value for its keys.

| Key | Meaning |
| --- | --- |
| `root` | Where to start in the output. Default: the whole document. |
| `available` | `{ "path": ... }` to a boolean. `false` shows "Plan limits don't apply to this account". |
| `plan` | `{ "path": ..., "names": { "prolite": "Pro Lite" } }` to a string. `names` is optional, up to 32 display names. |
| `access` | `{ "path": ... }` to a boolean. `false` shows "Usage limit reached". |
| `meters` | 1 to 16 meter specs. |

### Meters and windows

A meter is one limit (such as the plan, or one model's cap) with one or more time windows.

| Meter key | Meaning |
| --- | --- |
| scope | At most one of `select` (one nested value), `each` (every element of an array), `eachEntry` (every member of an object). Without one, the meter reads the current value. Missing or null values produce nothing. |
| `id` | Required. See Ids. |
| `label` | Required. See Labels. |
| `plan` | Optional, like the top-level `plan`. |
| `limitReachedReason` | Optional `{ "path": ... }` to text. |
| `windows` | 1 to 16 window specs. |

| Window key | Meaning |
| --- | --- |
| scope | `select`, `each`, or `eachEntry`, relative to the meter's value. |
| `id` | Required. See Ids. |
| `label` | Optional. Without it, the title comes from the duration ("Session" for 5 hours, "Weekly", "Monthly", or the length), then the meter's label. |
| `used` | Required. `{ "path": ..., "as": "percent" }` (100 is the full allowance), `"fraction"` (1 is the full allowance), or `"remainingFraction"` (0 to 1 left). Usage past the allowance is kept as overage. |
| `resetsAt` | Optional. `{ "path": ..., "as": "iso8601" }` (with a UTC offset such as `Z` or `+00:00`), `"epochSeconds"`, or `"epochMillis"`. |
| `duration` | Optional. `{ "seconds": 18000 }`, or `{ "path": ..., "as": "seconds" }`, `"minutes"`, or `"windowName"` (understands `five_hour`, `5h`, `daily`, `day`, `weekly`, `week`, `7d`). |

A window with neither a usage value nor a reset time is skipped, and so is a meter left without windows. When two meters or two windows resolve to the same id, the first is kept.

Ids are a literal string (1 to 64 letters, digits, `.`, `:`, `_`, or `-`), `{ "path": "/name" }` (a string or integer in the data), or `{ "entryKey": true }` (the member name under `eachEntry`).

Labels are `{ "text": "Opus" }`, `{ "key": "plan" }` (also `"session"`, `"weekly"`, `"monthly"`, translated by Maestri), or `{ "path": "/name", "fallback": { "text": "Codex" } }`, where the fallback is a `text`, a `key`, or `{ "entryKey": true }`.

### Rings

Up to three rings, outside in:

1. Session: the first meter's window of one day or less.
2. Weekly: the first meter's window longer than a day.
3. Special: the second meter's longest window, such as a model-specific cap.

A ring with nothing to show is left out. The first meter's windows need a `duration` to get a ring at all; the second meter's windows don't. The popover lists every meter and window either way.

## A minimal provider

A script that prints `{"week": {"percent": 42, "resets": 1790000000}}`:

```json
{
  "schemaVersion": 1,
  "id": "my-agent",
  "revision": 1,
  "name": "My Agent",
  "source": {
    "type": "command",
    "executable": "~/bin/my-agent-usage",
    "timeoutSeconds": 20
  },
  "map": {
    "meters": [
      {
        "id": "plan",
        "label": { "key": "plan" },
        "windows": [
          {
            "id": "week",
            "select": "/week",
            "used": { "path": "/percent", "as": "percent" },
            "resetsAt": { "path": "/resets", "as": "epochSeconds" },
            "duration": { "seconds": 604800 }
          }
        ]
      }
    ]
  }
}
```

## A second account

Copy `claude.json` to `claude-work.json`, set `"id": "claude-work"` and a new `name`, and point the CLI at the other account's config folder in `source`:

```json
"env": { "CLAUDE_CONFIG_DIR": "~/.claude-work" }
```

Codex reads `CODEX_HOME` the same way. Each copy gets its own rings and color.

## A remote machine over SSH

Copy the provider to a new id (for example `claude-devbox.json`) and run the CLI through `ssh`. Keep the steps and the map as they are.

```json
"executable": "ssh",
"args": [
  "-o", "BatchMode=yes",
  "-o", "ConnectTimeout=10",
  "devbox",
  "bash -lc 'claude -p --input-format stream-json --output-format stream-json --verbose --no-session-persistence'"
],
"timeoutSeconds": 45
```

- `ssh` joins everything after the host into one command line for the remote shell. Put the whole remote command in one argument, quoted as above; `"bash", "-lc", "claude -p ..."` as separate arguments would lose everything after `claude`.
- `bash -lc` loads the remote login profile, so the CLI is found where you installed it.
- Sign-in must work without a prompt. `BatchMode=yes` fails fast instead of waiting for a password or a new host key: connect once in Terminal first. A key file in `~/.ssh`, the SSH agent from `SSH_AUTH_SOCK`, or an `IdentityAgent` in `~/.ssh/config` (1Password, for example) all work. For an agent Maestri doesn't see, set its socket in `env`: `"SSH_AUTH_SOCK": "~/path/to/agent.sock"`.
- `stdio` providers ignore anything the remote shell prints before the answer. A `command` provider needs the remote side to print only the JSON.
- Each poll opens a connection. A `ControlMaster` and `ControlPersist` entry for the host in `~/.ssh/config` makes that fast.
- The rings show the remote machine's account, which can differ from the one on this Mac.

## Checking a provider

- After saving a file, read `.status.json` in this folder. Maestri rewrites it whenever a provider's state changes. For each provider it gives the `state` (`ready`, `failed`, `fileError`, `executableNotFound`, `waiting`, `running`, `off`, or `needsNewerMaestri`), the exact `error`, and the `executable` it ran. After a successful run it lists every meter and window as mapped, with `usedPercent`, `resetsAt`, `durationSeconds`, and the `ring` each window landed on, and `notes` lists anything the map skipped.
- A new or edited file loads within 15 seconds and runs as soon as it may: while the rings are on screen, and at most once a minute per provider. Editing a file clears the backoff from its earlier failures.
- To see the raw output, run the same command in Terminal. For `stdio`, pipe in the `write` lines, for example: `printf '%s\n' '{"type":"control_request","request_id":"maestri-usage","request":{"subtype":"get_usage","skip_behaviors":true}}' | claude -p --input-format stream-json --output-format stream-json --verbose --no-session-persistence`
- Exit code 127 usually means a program the CLI needs (such as Node.js) isn't on PATH. Exit code 255 from `ssh` means the connection or sign-in failed.