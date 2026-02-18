// const SLOPE: f64 = 0.5;

#[must_use]
pub fn linear_to_logarithmic(linear_percent: f64) -> f64 {
    if linear_percent <= 0.0 {
        return 0.0;
    }

    if linear_percent >= 100.0 {
        return 100.0;
    }

    // Slope < 1.0 = gentler curve, spreads out low values
    let normalized = linear_percent / 100.0;
    // 100.0 * (normalized.powf(SLOPE))
    100.0 * (normalized.sqrt())
}

#[must_use]
pub fn logarithmic_to_linear(log_percent: f64) -> f64 {
    if log_percent <= 0.0 {
        return 0.0;
    }

    if log_percent >= 100.0 {
        return 100.0;
    }

    let normalized = log_percent / 100.0;
    // 100.0 * normalized.powf(1.0 / SLOPE)
    100.0 * normalized.powi(2)
}
