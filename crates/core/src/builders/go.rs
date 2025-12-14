use crate::builders::{Builder, Image};
use std::path::Path;

pub struct GoBuilder;

impl GoBuilder {
    pub fn new() -> Self {
        GoBuilder
    }
}

#[async_trait::async_trait]
impl Builder for GoBuilder {
    async fn build(
        &self,
        build_path: &Path,
        image_name: &str,
        image_tag: &str,
    ) -> anyhow::Result<Image> {
        anyhow::bail!("unimplemented")
    }
}

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
