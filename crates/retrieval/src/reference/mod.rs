//! Independently implemented numerical contracts extracted from frozen VCP.
//! Neutral DTOs have no Authority, MemoryRevision or AssociationEvidence owner.
mod fields;
pub use fields::*;
mod sense;
pub use sense::*;
mod graph;
pub use graph::*;
mod pyramid;
pub use pyramid::*;
mod query_shape;
pub use query_shape::*;
