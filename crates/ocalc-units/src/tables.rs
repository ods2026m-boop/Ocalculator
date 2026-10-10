//! Built-in unit tables: length, mass, temperature, data, energy, time.

use crate::{UnitCategory, UnitConverter, UnitDefinition};

/// Build a converter pre-loaded with all built-in units.
#[must_use]
pub fn builtin_converter() -> UnitConverter {
    let mut c = UnitConverter::new();
    for def in builtin_units() {
        c.register(def);
    }
    c
}

/// All built-in unit definitions.
#[must_use]
pub fn builtin_units() -> Vec<UnitDefinition> {
    use UnitCategory as C;
    vec![
        // ---- Length (base: meter) ----
        UnitDefinition::linear("m", &["meter", "meters", "metre", "metres"], C::Length, 1.0),
        UnitDefinition::linear(
            "km",
            &["kilometer", "kilometers", "kilometre", "kilometres"],
            C::Length,
            1000.0,
        ),
        UnitDefinition::linear(
            "cm",
            &["centimeter", "centimeters", "centimetre", "centimetres"],
            C::Length,
            0.01,
        ),
        UnitDefinition::linear(
            "mm",
            &["millimeter", "millimeters", "millimetre", "millimetres"],
            C::Length,
            0.001,
        ),
        UnitDefinition::linear("um", &["micrometer", "micrometers", "µm"], C::Length, 1e-6),
        UnitDefinition::linear("nm", &["nanometer", "nanometers"], C::Length, 1e-9),
        UnitDefinition::linear("in", &["inch", "inches"], C::Length, 0.0254),
        UnitDefinition::linear("ft", &["foot", "feet"], C::Length, 0.3048),
        UnitDefinition::linear("yd", &["yard", "yards"], C::Length, 0.9144),
        UnitDefinition::linear("mi", &["mile", "miles"], C::Length, 1609.344),
        UnitDefinition::linear(
            "nmi",
            &["nautical_mile", "nautical_miles"],
            C::Length,
            1852.0,
        ),
        UnitDefinition::linear("au", &["astronomical_unit"], C::Length, 149_597_870_700.0),
        UnitDefinition::linear(
            "ly",
            &["light_year", "light_years", "lightyear", "lightyears"],
            C::Length,
            9.460_730_472_580_8e15,
        ),
        // ---- Mass (base: kilogram) ----
        UnitDefinition::linear("kg", &["kilogram", "kilograms"], C::Mass, 1.0),
        UnitDefinition::linear("g", &["gram", "grams"], C::Mass, 0.001),
        UnitDefinition::linear("mg", &["milligram", "milligrams"], C::Mass, 1e-6),
        UnitDefinition::linear("t", &["tonne", "tonnes", "metric_ton"], C::Mass, 1000.0),
        UnitDefinition::linear("lb", &["pound", "pounds", "lbs"], C::Mass, 0.453_592_37),
        UnitDefinition::linear("oz", &["ounce", "ounces"], C::Mass, 0.028_349_523_125),
        // ---- Temperature (base: kelvin, affine) ----
        UnitDefinition {
            symbol: "K",
            aliases: &["kelvin"],
            category: C::Temperature,
            factor_to_base: 1.0,
            offset_to_base: 0.0,
        },
        UnitDefinition {
            symbol: "C",
            aliases: &["celsius", "centigrade", "degC"],
            category: C::Temperature,
            factor_to_base: 1.0,
            offset_to_base: 273.15,
        },
        UnitDefinition {
            symbol: "F",
            aliases: &["fahrenheit", "degF"],
            category: C::Temperature,
            factor_to_base: 5.0 / 9.0,
            offset_to_base: 273.15 - 32.0 * 5.0 / 9.0,
        },
        // ---- Data (base: byte, decimal SI + binary IEC) ----
        UnitDefinition::linear("B", &["byte", "bytes"], C::Data, 1.0),
        UnitDefinition::linear("KB", &["kilobyte", "kilobytes"], C::Data, 1e3),
        UnitDefinition::linear("MB", &["megabyte", "megabytes"], C::Data, 1e6),
        UnitDefinition::linear("GB", &["gigabyte", "gigabytes"], C::Data, 1e9),
        UnitDefinition::linear("TB", &["terabyte", "terabytes"], C::Data, 1e12),
        UnitDefinition::linear("PB", &["petabyte", "petabytes"], C::Data, 1e15),
        UnitDefinition::linear("KiB", &["kibibyte", "kibibytes"], C::Data, 1024.0),
        UnitDefinition::linear("MiB", &["mebibyte", "mebibytes"], C::Data, 1024.0 * 1024.0),
        UnitDefinition::linear(
            "GiB",
            &["gibibyte", "gibibytes"],
            C::Data,
            1024.0 * 1024.0 * 1024.0,
        ),
        UnitDefinition::linear(
            "TiB",
            &["tebibyte", "tebibytes"],
            C::Data,
            1024.0_f64.powi(4),
        ),
        UnitDefinition::linear("bit", &["bits"], C::Data, 1.0 / 8.0),
        UnitDefinition::linear("kbit", &["kilobit"], C::Data, 1e3 / 8.0),
        // ---- Energy (base: joule) ----
        UnitDefinition::linear("J", &["joule", "joules"], C::Energy, 1.0),
        UnitDefinition::linear("kJ", &["kilojoule", "kilojoules"], C::Energy, 1e3),
        UnitDefinition::linear("cal", &["calorie", "calories"], C::Energy, 4.184),
        UnitDefinition::linear("kcal", &["kilocalorie", "kilocalories"], C::Energy, 4184.0),
        UnitDefinition::linear("Wh", &["watt_hour", "watthour"], C::Energy, 3600.0),
        UnitDefinition::linear("kWh", &["kilowatt_hour"], C::Energy, 3.6e6),
        UnitDefinition::linear("eV", &["electronvolt"], C::Energy, 1.602_176_634e-19),
        UnitDefinition::linear(
            "BTU",
            &["btu", "british_thermal_unit"],
            C::Energy,
            1_055.055_852_62,
        ),
        // ---- Time (base: second) ----
        UnitDefinition::linear("s", &["sec", "second", "seconds"], C::Time, 1.0),
        UnitDefinition::linear("ms", &["millisecond", "milliseconds"], C::Time, 1e-3),
        UnitDefinition::linear("us", &["microsecond", "microseconds"], C::Time, 1e-6),
        UnitDefinition::linear("ns", &["nanosecond", "nanoseconds"], C::Time, 1e-9),
        UnitDefinition::linear("min", &["minute", "minutes"], C::Time, 60.0),
        UnitDefinition::linear("h", &["hr", "hour", "hours"], C::Time, 3600.0),
        UnitDefinition::linear("day", &["days"], C::Time, 86400.0),
        UnitDefinition::linear("week", &["weeks"], C::Time, 604_800.0),
        UnitDefinition::linear("year", &["years", "yr"], C::Time, 31_557_600.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv() -> UnitConverter {
        builtin_converter()
    }

    #[test]
    fn length() {
        assert!((conv().convert(1.0, "km", "m").unwrap() - 1000.0).abs() < 1e-9);
        assert!((conv().convert(1.0, "mi", "km").unwrap() - 1.609344).abs() < 1e-9);
        assert!((conv().convert(12.0, "in", "ft").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mass() {
        assert!((conv().convert(1.0, "kg", "g").unwrap() - 1000.0).abs() < 1e-9);
        assert!((conv().convert(1.0, "lb", "kg").unwrap() - 0.45359237).abs() < 1e-9);
    }

    #[test]
    fn temperature_affine() {
        assert!((conv().convert(0.0, "C", "K").unwrap() - 273.15).abs() < 1e-9);
        assert!((conv().convert(32.0, "F", "C").unwrap()).abs() < 1e-9);
        assert!((conv().convert(100.0, "C", "F").unwrap() - 212.0).abs() < 1e-9);
        assert!((conv().convert(0.0, "K", "C").unwrap() + 273.15).abs() < 1e-9);
    }

    #[test]
    fn data_decimal_and_binary() {
        assert_eq!(conv().convert(1.0, "KB", "B").unwrap(), 1000.0);
        assert_eq!(conv().convert(1.0, "KiB", "B").unwrap(), 1024.0);
        assert_eq!(conv().convert(8.0, "bit", "B").unwrap(), 1.0);
    }

    #[test]
    fn energy_and_time() {
        assert!((conv().convert(1.0, "kWh", "J").unwrap() - 3.6e6).abs() < 1e-3);
        assert_eq!(conv().convert(1.0, "h", "s").unwrap(), 3600.0);
        assert_eq!(conv().convert(1.0, "day", "h").unwrap(), 24.0);
    }
}
