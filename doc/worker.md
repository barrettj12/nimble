# Workers

The Nimble agent runs background workers to handle tasks (such as builds and deployments). Workers are stored in `crates/agent/src/workers`. Workers generally have two public methods:
- `new`: creates a new worker. Arguments include worker dependencies such as:
  - the agent config
  - the database
  - any outgoing channels (e.g. for the build worker, we pass in the deploy channel to queue deployments)
- `run`: spawns a loop which reads jobs off a channel and processes each in turn. It accepts the input channel as an argument (e.g. the build queue for the build worker).
