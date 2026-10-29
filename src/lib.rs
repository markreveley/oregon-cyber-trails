//! Oregon Cyber Trails: an Oregon Trail-style game about defending a
//! company's identity and security. The engine is pure (no I/O) so the same
//! code drives the terminal, a browser build and the balancing bots.

pub mod bots;
pub mod content;
pub mod game;
pub mod logs;
pub mod rng;
pub mod text;

pub use game::{Company, Ctx, Ending, Game, Level, Tombstone};
