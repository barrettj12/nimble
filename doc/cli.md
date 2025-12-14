# Nimble CLI

The `nimble` CLI allows the user to communicate remotely with the Nimble agent `nimbled`.

## Global flags

All commands accept `--agent-url` (default `http://localhost:7080`).

## Deploy source

```
nimble deploy <directory> [--wait] [--agent-url <url>]
```

- Archives `<directory>` into a `.tar.gz` and uploads it as a new build.
- `--wait` keeps polling until the build succeeds or fails.

## List builds

```
nimble build list [--status <filter>] [--limit <n>] [--agent-url <url>]
```

- Shows a table of recent builds.
- `--status` filters (queued, building, success, failed).
- `--limit` caps row count.

## Get build details

```
nimble build get <build_id> [--agent-url <url>]
```

- Displays the status and timestamps for a single build.
