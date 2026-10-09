// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fs, path::Path};

fn sources(root: &Path, path: &Path, files: &mut BTreeMap<String, std::path::PathBuf>) {
    if path.is_dir() {
        println!("cargo:rerun-if-changed={}", path.display());
        for entry in fs::read_dir(path).expect("projection sources") {
            sources(root, &entry.expect("projection source entry").path(), files);
        }
    } else {
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        files.insert(name, path.to_owned());
    }
}

fn main() {
    // Projection builders and their input owners determine asset compatibility.
    // Catalog presentation and runtime query policy are deliberately separate.
    let root = Path::new("../..");
    let mut files = BTreeMap::new();
    for path in [
        "Cargo.lock",
        "crates/retrieval/build.rs",
        "crates/retrieval/src/artifacts.rs",
        "crates/retrieval/src/build.rs",
        "crates/retrieval/src/concept_generation.rs",
        "crates/retrieval/src/dense.rs",
        "crates/retrieval/src/exact.rs",
        "crates/retrieval/src/graph.rs",
        "crates/retrieval/src/lexical.rs",
        "crates/retrieval/src/material.rs",
        "crates/retrieval/src/provider.rs",
        "crates/retrieval/src/residual.rs",
        "crates/retrieval/src/vcp_adapter.rs",
        "crates/retrieval/src/vcp_generation.rs",
        "crates/retrieval/src/vcp_graph.rs",
        "crates/retrieval/src/vcp_index.rs",
        "crates/retrieval/src/vcp_material.rs",
        "crates/retrieval/src/reference",
        "crates/core/src/concept.rs",
        "crates/persistence/src/projection_input.rs",
        "crates/persistence/src/historical_projection.rs",
        "crates/persistence/src/topology_input.rs",
    ] {
        sources(root, &root.join(path), &mut files);
    }
    let mut hash = blake3::Hasher::new();
    for (name, path) in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let content = fs::read_to_string(&path)
            .expect("projection source UTF-8")
            .replace("\r\n", "\n");
        hash.update(&(name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update(&(content.len() as u64).to_le_bytes());
        hash.update(content.as_bytes());
    }
    println!(
        "cargo:rustc-env=NOUS_SERVING_IMPLEMENTATION_DIGEST={}",
        hash.finalize().to_hex()
    );
}
