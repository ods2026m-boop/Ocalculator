//! Stable numeric value alias (Phase 3 public API).
//!
//! [`Value`] is the single result type every frontend handles. It is a
//! re-export of [`ocalc_number::Number`] so the engine and all modules share
//! one exact representation (Integer/BigInt/Rational/Decimal/Float/Complex).
//! The alias exists so Iced/CLI/ODS-os code depends on `ocalc_engine::Value`
//! — the stable path — instead of reaching into `ocalc-number` directly.

/// Stable calculator value (exact number).
///
/// See [`ocalc_number::Number`] for the precision model. `Value` adds JSON
/// helpers with the Phase 3 v1 format so results can be persisted in history
/// files without precision loss.
pub type Value = ocalc_number::Number;

/// Format version for [`value_to_json`] / [`value_from_json`] envelopes.
pub const VALUE_FORMAT_VERSION: u32 = 1;

/// Serialize a [`Value`] with a version envelope.
///
/// Produces `{"format_version":1,"value":{…}}`.
#[must_use]
pub fn value_to_json_string(value: &Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "format_version": VALUE_FORMAT_VERSION,
        "value": value,
    }))
    .unwrap_or_else(|_| {
        "{\"format_version\":1,\"value\":{\"kind\":\"float\",\"value\":\"NaN\"}}".to_owned()
    })
}

/// Deserialize [`value_to_json_string`] output (also accepts a bare value
/// object for forward flexibility).
///
/// # Errors
/// JSON parse errors as strings.
pub fn value_from_json_string(s: &str) -> Result<Value, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| format!("invalid value JSON: {e}"))?;
    // Enveloped `{"format_version":1,"value":{…}}` vs bare `{"kind":…}`.
    let inner = if v.get("format_version").is_some() {
        v.get("value").unwrap_or(&v)
    } else {
        &v
    };
    serde_json::from_value::<Value>(inner.clone()).map_err(|e| format!("bad value payload: {e}"))
}

/// Short type name for UI display (`"integer"`, `"float"`, …).
#[must_use]
pub fn value_kind_name(value: &Value) -> &'static str {
    value.kind()
}

/// Re-exported for generic bounds in frontend code.
pub trait ValueExt {
    /// Lossy `f64` for rendering (complex → magnitude).
    fn to_display_f64(&self) -> f64;
    /// Exact string for history/result panes.
    fn to_result_string(&self) -> String;
}

impl ValueExt for Value {
    fn to_display_f64(&self) -> f64 {
        self.to_f64()
    }
    fn to_result_string(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocalc_number::Number;

    #[test]
    fn json_envelope_roundtrips_all_variants() {
        let cases = vec![
            Number::Integer(-42),
            Number::from_bigint(ocalc_number::BigInt::from(10).pow(100)),
            Number::Integer(1).div(&Number::Integer(3)).unwrap(),
            Number::from_decimal_str("0.3").unwrap(),
            Number::Float(f64::INFINITY),
            Number::Float(f64::NAN),
            Number::complex(1.0, 2.0),
        ];
        for v in cases {
            let s = value_to_json_string(&v);
            let back = value_from_json_string(&s).unwrap();
            if v.to_string() == "NaN" {
                assert!(back.to_f64().is_nan(), "NaN roundtrip");
            } else {
                assert_eq!(back, v, "roundtrip for {v}");
            }
        }
    }

    #[test]
    fn bare_value_accepted() {
        let bare = serde_json::to_string(&Number::Integer(7)).unwrap();
        assert_eq!(value_from_json_string(&bare).unwrap(), Number::Integer(7));
    }
}
