//! Decision utility primitives — xG, xT, pitch control, pressing, softmax.
//!
//! These are the six building blocks behind every BT node's `utility()`
//! score.  All arithmetic is Q32; no floats enter canonical state.

pub mod pitch_control;
pub mod pressing;
pub mod softmax;
pub mod xg;
pub mod xt;
