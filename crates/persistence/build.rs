// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

fn main() {
    // SQLx embeds migrations at compile time; adding a file must invalidate it.
    println!("cargo:rerun-if-changed=migrations");
}
