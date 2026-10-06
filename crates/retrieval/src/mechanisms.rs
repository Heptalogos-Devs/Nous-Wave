//! Read-only serving adapters, bounded retrieval and evidence-aware ranking.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
pub type SparseField = BTreeMap<u32, f64>;

#[path = "cue_sensing.rs"]
mod cue_sensing;
#[path = "dense.rs"]
pub mod dense;
#[path = "exact.rs"]
mod exact;
#[path = "fields.rs"]
mod fields;
#[path = "graph.rs"]
mod graph;
#[path = "lexical.rs"]
pub mod lexical;
#[path = "residual.rs"]
pub mod residual;
#[path = "seams.rs"]
mod seams;
#[path = "serving.rs"]
mod serving;
#[path = "wave.rs"]
mod wave;

pub use cue_sensing::*;
pub use dense::{DenseGeneration, DenseMatch, VectorRecord};
pub use exact::*;
pub use fields::*;
pub use graph::*;
pub use lexical::{LexicalDocument, LexicalGeneration, LexicalMatch};
pub use residual::*;
pub use seams::*;
pub use serving::*;
pub use wave::*;
