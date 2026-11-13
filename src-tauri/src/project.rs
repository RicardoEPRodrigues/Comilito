//! The project data model. Only the sound categories so far.

use serde::{Deserialize, Serialize};

/// The three kinds of sound. Each has its own mixer track and volume.
///
/// A Rust `enum` like this one is a closed set of values, like a C# enum. `match` on it must
/// handle every variant, so adding a fourth category later makes the compiler point at
/// every place that needs updating.
///
/// The `derive` line asks the compiler to generate common behaviour:
/// - `Clone, Copy`: copying a Category is as cheap as copying an int, so it's passed by value.
/// - `PartialEq, Eq`: allows `==`.
/// - `Debug`: printable with `{:?}` for logging.
/// - `Serialize, Deserialize` (from serde): converts to and from JSON.
///   `rename_all = "lowercase"` makes `Category::Music` appear as `"music"` in JSON, which is
///   what the frontend sends and what comilito.json uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Music,
    Ambience,
    Effects,
}
