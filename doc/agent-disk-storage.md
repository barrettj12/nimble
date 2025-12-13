This document describes how the Nimble agent `nimbled` stores artifacts on disk, including source code archives, build directories, database files, etc.

By default, when running on a production machine, `nimbled` stores all its artifacts in `/var/lib/nimble/`. If running in development, e.g. on a dev machine without root access, it can instead store its data in `./data/` or `$XDG_DATA_HOME/nimble/`.

The storage base path is configurable via the `AgentConfig` struct:
```rust
pub struct AgentConfig {
    pub data_dir: PathBuf,
}
```


## Directory layout

```text
$NIMBLE_DIR
├── db/
│   └── nimble.db
├── artifacts/
│   ├── source/
│   │   ├── build-<id>.tar.gz
│   │   └── build-<id>/
│   └── image/
│       └── build-<id>.tar
├── builds/
│   └── build-<id>/
│       ├── workspace/
│       ├── logs.txt
│       └── result.json
└── tmp/
```

## What goes where

### 📦 Zipped source code

```
/var/lib/nimble/artifacts/source/build-<id>.tar.gz
```

* Immutable
* Stored once
* Useful for debugging & replays

### 📂 Unzipped source (build workspace)

```
/var/lib/nimble/builds/build-<id>/workspace/
```

* Ephemeral
* Can be deleted after build
* Safe to mutate

### 🐳 Built images

Generally, we don't need to store image blobs ourselves - we can let Docker/`containerd` handle the storage.

If we later find that we do need raw image storage, we could use the location:

```
/var/lib/nimble/artifacts/image/build-<id>.tar
```

### 🧪 Temporary files

```
/var/lib/nimble/tmp/
```

* Scratch space
* Cleaned periodically
* Safe to delete on restart