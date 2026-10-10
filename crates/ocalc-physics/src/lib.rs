//! `ocalc-physics`: physics toolkit for Ocalculator.
//!
//! SI constants, mechanics and electricity as native Rust functions, bridged
//! into engine expressions by [`PhysicsModule`] (`force()`, `energy()`…).
//! Non-SI inputs convert first through the existing unit system
//! (`kmh_to_ms`, `ms_to_kmh`).

mod constants;
mod electricity;
mod mechanics;
mod module;

pub use constants::{
    ALL as CONSTANTS, AVOGADRO, BOLTZMANN, ELEMENTARY_CHARGE, GRAVITY, PLANCK, SPEED_OF_LIGHT,
};
pub use electricity::{current, power_ri, power_vi, power_vr, resistance, voltage};
pub use mechanics::{
    acceleration, force, kinetic_energy, kmh_to_ms, momentum, ms_to_kmh, potential_energy,
    velocity, weight,
};
pub use module::{PhysicsModule, PhysicsProvider};
