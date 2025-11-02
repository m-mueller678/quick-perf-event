use std::fmt;

/// A wrapper type for rendering floating-point numbers in compact, readable form for tables.
///
/// `TabledFloat` formats values with fixed width (7 characters)
/// and uses SI unit prefixes (`m`, `µ`, `n`, `p`, `k`, `M`, `G`, `T`).
/// It is intended to allow easy comparison of values across rows while remaining compact.
/// For values with a reasonable magnitude, it shows at least two digits of precision.
/// This is the format used for the `live` output format.
///
/// # Formatting rules
/// - Negative and non-finite values use an unspecified format with appropriate width
/// - Values close to 1 are printed as fixed-point with three decimals (`100.000`, `0.010`)
/// - Larger or smaller magnitudes are scaled with SI prefixes, leaving only one digit past the decimal point (`1.0 k`, `500.0 µ`)
/// - Very large values are formatted using scientific notation (`5e42`)
/// - Very small values are rounded down to 0 (`0`)
pub struct TabledFloat(pub f64);

impl fmt::Display for TabledFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale = self.0.log10().floor() as isize;
        let si_scale = scale.div_euclid(3);
        if !self.0.is_finite() || self.0.is_sign_negative() {
            write!(f, "{:7.0e}", self.0)
        } else if scale >= -2 && scale <= 2 {
            write!(f, "{:7.3}", self.0)
        } else {
            if si_scale > 0 {
                if let Some(suffix) = ["k", "M", "G", "T"].get(si_scale as usize - 1) {
                    let scaled = self.0 / (1000f64).powi(si_scale as i32);
                    write!(f, "{scaled:5.1} {suffix}")
                } else {
                    write!(f, "{:7e}", self.0)
                }
            } else {
                if let Some(suffix) = ["m", "µ", "n", "p"].get(-si_scale as usize - 1) {
                    let scaled = self.0 / (1000f64).powi(si_scale as i32);
                    write!(f, "{scaled:5.1} {suffix}")
                } else {
                    write!(f, "{:7}", 0)
                }
            }
        }
    }
}

#[test]
fn test_fixed_float() {
    let cases = [
        (1e-20, "      0"),
        (1e-6, "  1.0 µ"),
        (5e-6, "  5.0 µ"),
        (1e-5, " 10.0 µ"),
        (5e-5, " 50.0 µ"),
        (1e-4, "100.0 µ"),
        (5e-4, "500.0 µ"),
        (1e-3, "  1.0 m"),
        (1e-2, "  0.010"),
        (5e-2, "  0.050"),
        (1e-1, "  0.100"),
        (5e-1, "  0.500"),
        (1e+0, "  1.000"),
        (5e+0, "  5.000"),
        (1e+1, " 10.000"),
        (5e+1, " 50.000"),
        (1e+2, "100.000"),
        (5e+2, "500.000"),
        (1e+3, "  1.0 k"),
        (5e+3, "  5.0 k"),
        (1e+4, " 10.0 k"),
        (5e+42, "   5e42"),
        (1e+105, "  1e105"),
    ];
    for x in cases {
        assert_eq!(TabledFloat(x.0).to_string(), x.1);
    }
}

#[test]
fn test_fixed_float_special() {
    let cases = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        0f64,
        -0f64,
        f64::EPSILON / 4.0,
        -f64::EPSILON / 4.0,
    ];
    for x in cases {
        assert!(
            TabledFloat(x).to_string().len() == 7,
            "bad length: {}",
            TabledFloat(x)
        );
    }
}
