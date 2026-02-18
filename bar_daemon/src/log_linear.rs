#[must_use]
pub fn linear_to_logarithmic(linear_percent: f64) -> f64 {
    if linear_percent <= 0.0 {
        return 0.0;
    }

    if linear_percent >= 100.0 {
        return 100.0;
    }

    let normalized = linear_percent / 100.0;
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
    100.0 * normalized.powi(2)
}

#[cfg(test)]
#[test]
fn log_linear_test() {
    for linear in 0..=100 {
        // Convert from linear to logarithmic and back
        let log = linear_to_logarithmic(f64::from(linear));
        let returned = logarithmic_to_linear(log);

        // Check with 32-bit precision if the values are equal
        assert!(
            (returned as f32).partial_cmp(&(f64::from(linear) as f32)) == Some(std::cmp::Ordering::Equal),
            "Linear -> Logarithmic -> Linear Conversion Fail: (Linear: {linear}\tLog: {log})\tReturned: {}",
            returned as f32
        );
    }

    for log in 0..=100 {
        // Convert from logarithmic to linear and back
        let linear = logarithmic_to_linear(f64::from(log));
        let returned = linear_to_logarithmic(linear);

        // Check with 32-bit precision if the values are equal
        assert!(
            (returned as f32).partial_cmp(&(f64::from(log) as f32)) == Some(std::cmp::Ordering::Equal),
            "Logarithmic -> Linear -> Logarithmic Conversion Fail: (Log: {log}\t Linear: {linear})\tReturned: {}",
            returned as f32
        );
    }
}
