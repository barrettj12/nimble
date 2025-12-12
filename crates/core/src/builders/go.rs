use std::fmt::write;

use crate::{builders::Builder, types::Image};
use tempfile::NamedTempFile;

pub struct GoBuilder;

impl GoBuilder {
    pub fn new() -> Self {
        GoBuilder
    }
}

impl Builder for GoBuilder {
    fn build(&self, path: &std::path::Path) -> Result<Image, Box<dyn std::error::Error>> {
        // Create temp directory
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, DOCKERFILE)?;

        // Copy project over to tmp directory

        // Docker build
        // docker build -t my-image -f /path/to/Dockerfile /path/to/build/context

        println!("Building Go project");
        Ok(Image)
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
