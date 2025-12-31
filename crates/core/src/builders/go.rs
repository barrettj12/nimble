use std::path::Path;

use crate::builders::{Builder, Image};

pub struct GoBuilder;

impl GoBuilder {
    pub fn new() -> Self {
        GoBuilder
    }
}

impl Default for GoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Builder for GoBuilder {
    async fn build(
        &self,
        _build_path: &Path,
        _image_name: &str,
        _image_tag: &str,
    ) -> anyhow::Result<Image> {
        anyhow::bail!("unimplemented")
    }
}

#[allow(dead_code)]
const DOCKERFILE: &str = r#"
# Stage 1: build the binary
FROM golang:1.22-alpine AS builder
WORKDIR /src
COPY . .
RUN go build -o myapp .

# Stage 2: minimal image
FROM alpine:3.18
COPY --from=builder /src/myapp /usr/local/bin/myapp
ENTRYPOINT ["myapp"]
"#;
