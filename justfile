check:
  cargo check --target wasm32-unknown-unknown --lib

clippy:
  cargo  clippy --tests

format:
    cargo +nightly fmt

test:
  cargo test

optimize:
  if [[ $(uname -m) =~ "arm64" ]]; then \
    just optimize-arm; else \
    just optimize-x86; fi

optimize-arm:
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/arm64 \
    cosmwasm/workspace-optimizer-arm64:0.15.1

optimize-x86:
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    cosmwasm/workspace-optimizer:0.15.1
