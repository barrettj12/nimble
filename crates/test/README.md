# Nimble end-to-end tests

This crate hosts the opt-in end-to-end test that drives the full stack (`nimbled` agent + `nimble` CLI) against the sample project in `examples/go-hello`.

## What it does
- Builds the `nimble` and `nimbled` binaries.
- Starts `nimbled` on port 7080 with a temp data directory.
- Calls `nimble deploy --wait` against the agent.
- Confirms the build completes successfully and cleans up the built image.

## Requirements
- Docker available on the PATH.
- Port `7080` free on localhost.

## Running
```sh
cargo test -- --nocapture
```
