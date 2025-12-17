# Nimble

Nimble is a PaaS (Platform-as-a-Service) system which allows developers to easily build and deploy their applications in the cloud without worrying about the complexities of infrastructure. Nimble takes care of the build and deployment process via a remote agent running on the destination machine, which the developer can communicate with using the `nimble` CLI tool. Application configuration is done using a `nimble.yaml` file in the project root, which provides a lightweight way to customise build and deployment options. 

Nimble is currently in the prototyping / early development stages - it is not yet feature complete or stable, and things are subject to change without notice.

## Getting started

Build the agent and CLI binaries:
```console
$ cargo build --workspace
```

Start the agent:
```console
$ target/debug/nimbled
nimbled listening on port 7080
```

Deploy an application:
```console
$ target/debug/nimble deploy examples/go-hello --wait
```

## Architecture

Nimble is composed of the following parts:
- The agent `nimbled`, which runs on the destination machine and receives requests from the client. The agent handles the build and deployment of applications.
- The CLI client `nimble`, which runs on the developer's machine and communicates with the remote agent.

The agent serves a public API, on port 7080 by default. Internally, it runs background workers which handle various tasks such as building and deploying applications.

From end to end, a deployment looks like this:
- The developer runs `nimble deploy <dir>`.
- The client compresses the project source directory into a tarball, and sends this over the wire to the `/builds` API.
- On the target machine, the agent's API handler saves the tarball to disk, and places the project in the build queue.
- The build worker picks the project off the queue, and decompresses the tarball into a working directory.
- It selects the correct builder and runs `docker build` to build an OCI image.
- The deployer worker then determines the deploy target and deploys the image accordingly. (yet to be implemented)

## Tech stack
Both the agent and the CLI are built in Rust. We use:
- Tokio for async
- Axum to build the HTTP server
- SQLite for persistence of agent data

## Repo structure

This repo consists of three crates:
- `crates/agent`: the Nimble agent `nimbled`.
- `crates/cmd`: the Nimble CLI `nimble`.
- `crates/core`: common logic shared between the agent and CLI.

Other important directories include:
- `doc`: contains reference documentation explaining the design and architecture of the system in more detail.
- `examples`: contains example Nimble projects, which can also be used for testing.

## Project configuration

Nimble projects can be configured using a `nimble.yaml` file in the project root. This is a simple key-value map which tells Nimble which builder to use, where to deploy the app, etc.

In the absence of a `nimble.yaml`, Nimble will attempt to automatically determine what builder to use to build the project. If a Dockerfile is present in the project root, it will default to using the Dockerfile builder. Otherwise, it will try to detect the language/framework of the project and use an appropriate builder.

## Development tips

Environment variables can be used to modify the behaviour of Nimble when running in a dev environment. Use
```
export NIMBLE_DEV_MODE=1
```
to enable "dev mode". Use
```
export NIMBLE_DATA_DIR=$(pwd)/.nimbledata
```
to set the Nimble data directory (where Nimble stores its DB, artifacts, builds, etc).

## Roadmap

See https://github.com/barrettj12/nimble/issues

## License

This project is open-source under the [MIT License](LICENSE). Contributions are welcomed :)
