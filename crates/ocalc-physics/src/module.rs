//! `physics` plugin module: constants + formulas as engine functions.

use std::sync::Arc;

use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

use crate::{constants, electricity, mechanics};

/// Provider names (lowercase, sorted).
const FUNCTIONS: [&str; 23] = [
    "acceleration",
    "avogadro",
    "boltzmann",
    "current",
    "elementary_charge",
    "energy",
    "force",
    "gravity",
    "kinetic_energy",
    "kmh_to_ms",
    "momentum",
    "ms_to_kmh",
    "planck",
    "potential_energy",
    "power",
    "power_ri",
    "power_vi",
    "power_vr",
    "resistance",
    "speed_of_light",
    "velocity",
    "voltage",
    "weight",
];

/// Physics kernels as a [`FunctionProvider`] (all SI; see each function).
#[derive(Debug, Clone, Default)]
pub struct PhysicsProvider;

impl PhysicsProvider {
    /// Shared instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl FunctionProvider for PhysicsProvider {
    fn function_names(&self) -> Vec<String> {
        FUNCTIONS.iter().map(|s| (*s).to_owned()).collect()
    }

    fn provides(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        FUNCTIONS.iter().any(|f| *f == key)
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        let key = name.to_ascii_lowercase();
        let v: Vec<f64> = args.iter().map(|n| n.to_f64()).collect();
        let need = |n: usize, usage: &str| {
            if v.len() != n {
                Some(Err(usage.to_owned()))
            } else {
                None
            }
        };
        match key.as_str() {
            "gravity" if v.is_empty() => Some(Ok(Number::from_f64(constants::GRAVITY))),
            "speed_of_light" if v.is_empty() => {
                Some(Ok(Number::from_f64(constants::SPEED_OF_LIGHT)))
            }
            "planck" if v.is_empty() => Some(Ok(Number::from_f64(constants::PLANCK))),
            "avogadro" if v.is_empty() => Some(Ok(Number::from_f64(constants::AVOGADRO))),
            "boltzmann" if v.is_empty() => Some(Ok(Number::from_f64(constants::BOLTZMANN))),
            "elementary_charge" if v.is_empty() => {
                Some(Ok(Number::from_f64(constants::ELEMENTARY_CHARGE)))
            }
            "force" => need(2, "force(mass_kg, accel_ms2)")
                .or_else(|| Some(Ok(Number::from_f64(mechanics::force(v[0], v[1]))))),
            "velocity" => need(2, "velocity(distance_m, time_s)")
                .or_else(|| Some(mechanics::velocity(v[0], v[1]).map(Number::from_f64))),
            "weight" => need(1, "weight(mass_kg)")
                .or_else(|| Some(Ok(Number::from_f64(mechanics::weight(v[0]))))),
            "acceleration" => need(2, "acceleration(delta_v, time_s)")
                .or_else(|| Some(mechanics::acceleration(v[0], v[1]).map(Number::from_f64))),
            "momentum" => need(2, "momentum(mass_kg, velocity_ms)")
                .or_else(|| Some(Ok(Number::from_f64(mechanics::momentum(v[0], v[1]))))),
            "kinetic_energy" | "energy" => need(2, "kinetic_energy(mass_kg, velocity_ms)")
                .or_else(|| Some(mechanics::kinetic_energy(v[0], v[1]).map(Number::from_f64))),
            "potential_energy" => need(2, "potential_energy(mass_kg, height_m)").or_else(|| {
                Some(Ok(Number::from_f64(mechanics::potential_energy(
                    v[0], v[1],
                ))))
            }),
            "kmh_to_ms" => need(1, "kmh_to_ms(v)")
                .or_else(|| Some(Ok(Number::from_f64(mechanics::kmh_to_ms(v[0]))))),
            "ms_to_kmh" => need(1, "ms_to_kmh(v)")
                .or_else(|| Some(Ok(Number::from_f64(mechanics::ms_to_kmh(v[0]))))),
            "voltage" => need(2, "voltage(current_a, resistance_ohm)")
                .or_else(|| Some(Ok(Number::from_f64(electricity::voltage(v[0], v[1]))))),
            "current" => need(2, "current(voltage_v, resistance_ohm)")
                .or_else(|| Some(electricity::current(v[0], v[1]).map(Number::from_f64))),
            "resistance" => need(2, "resistance(voltage_v, current_a)")
                .or_else(|| Some(electricity::resistance(v[0], v[1]).map(Number::from_f64))),
            "power_vi" | "power" => need(2, "power_vi(voltage_v, current_a)")
                .or_else(|| Some(Ok(Number::from_f64(electricity::power_vi(v[0], v[1]))))),
            "power_ri" => need(2, "power_ri(current_a, resistance_ohm)")
                .or_else(|| Some(Ok(Number::from_f64(electricity::power_ri(v[0], v[1]))))),
            "power_vr" => need(2, "power_vr(voltage_v, resistance_ohm)")
                .or_else(|| Some(electricity::power_vr(v[0], v[1]).map(Number::from_f64))),
            _ => None,
        }
    }
}

/// `physics` calculator module.
#[derive(Debug, Clone)]
pub struct PhysicsModule {
    provider: Arc<PhysicsProvider>,
}

impl Default for PhysicsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsModule {
    /// New module.
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: Arc::new(PhysicsProvider::new()),
        }
    }
}

impl CalculatorModule for PhysicsModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "physics",
            env!("CARGO_PKG_VERSION"),
            "physics toolkit: constants, mechanics, electricity (SI)",
        )
        .with_capability(Capability::Custom("physics".into()))
    }

    fn init(&self) -> Result<(), String> {
        // Sanity: F = ma and unit-table helpers agree on 36 km/h = 10 m/s.
        if mechanics::force(2.0, 3.0) != 6.0 {
            return Err("physics self-test failed: force".to_owned());
        }
        if (mechanics::kmh_to_ms(36.0) - 10.0).abs() > 1e-9 {
            return Err("physics self-test failed: unit tables".to_owned());
        }
        Ok(())
    }

    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self.provider.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocalc_modules::ModuleRegistry;

    fn eval(name: &str, args: &[f64]) -> Result<f64, String> {
        let p = PhysicsProvider::new();
        let nums: Vec<Number> = args.iter().map(|v| Number::from_f64(*v)).collect();
        p.call(name, &nums)
            .unwrap_or(Err(format!("{name} not provided")))
            .map(|n| n.to_f64())
    }

    #[test]
    fn provider_dispatch() {
        assert_eq!(eval("gravity", &[]).unwrap(), constants::GRAVITY);
        assert_eq!(eval("force", &[2.0, 3.0]).unwrap(), 6.0);
        assert_eq!(eval("velocity", &[100.0, 10.0]).unwrap(), 10.0);
        assert_eq!(eval("weight", &[10.0]).unwrap(), 10.0 * constants::GRAVITY);
        assert_eq!(eval("kinetic_energy", &[2.0, 3.0]).unwrap(), 9.0);
        assert_eq!(eval("energy", &[2.0, 3.0]).unwrap(), 9.0);
        assert_eq!(eval("voltage", &[2.0, 3.0]).unwrap(), 6.0);
        assert_eq!(eval("power_vi", &[6.0, 2.0]).unwrap(), 12.0);
        assert_eq!(eval("power", &[6.0, 2.0]).unwrap(), 12.0);
        assert_eq!(eval("power_ri", &[2.0, 3.0]).unwrap(), 12.0);
        assert_eq!(eval("power_vr", &[6.0, 3.0]).unwrap(), 12.0);
        assert_eq!(eval("kmh_to_ms", &[36.0]).unwrap(), 10.0);
        assert!(eval("force", &[1.0]).is_err());
        assert!(eval("velocity", &[1.0, 0.0]).is_err());
        assert!(eval("current", &[1.0, 0.0]).is_err());
        assert!(eval("power_vr", &[1.0, 0.0]).is_err());
        assert!(eval("nope", &[]).is_err());
        assert!(eval("gravity", &[1.0]).is_err());
    }

    #[test]
    fn registry_loading() {
        let mut r = ModuleRegistry::new();
        r.register(Arc::new(PhysicsModule::new())).unwrap();
        assert!(r.provides_function("force"));
        let meta = PhysicsModule::new().metadata();
        meta.validate().unwrap();
        meta.check_compatibility(ocalc_modules::ENGINE_PLUGIN_API_VERSION)
            .unwrap();
    }
}
