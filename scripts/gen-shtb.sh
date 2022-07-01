#!/usr/bin/sh
cargo run \
      -q --release \
      --manifest-path ./blockfish-engine/Cargo.toml \
      --features gen-shtb \
      --bin blockfish-gen-shtb \
      > ./support/test/srs-shape-table.json

cargo run \
      -q --release \
      --manifest-path ./blockfish-engine/Cargo.toml \
      --features gen-shtb \
      --bin blockfish-gen-shtb-trs \
      > ./support/test/trs-shape-table.json