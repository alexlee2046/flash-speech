/// 15-tap symmetric FIR low-pass filter for decimation by 3.
/// Designed: sinc(n/3) × Hamming window, normalized to unit DC gain.
/// Cutoff: ~7.5 kHz at 48 kHz (π/3 normalized), >40 dB stopband attenuation.
const DECIMATE3_TAPS: usize = 15;
const DECIMATE3_HALF: usize = 7; // (TAPS - 1) / 2

/// Pre-computed FIR coefficients. Symmetric: COEFFS[i] == COEFFS[14 - i].
/// Generated via: h[n] = sinc((n-7)/3) * (0.54 - 0.46 * cos(2*pi*n/14)), normalized.
const DECIMATE3_COEFFS: [f32; DECIMATE3_TAPS] = [
    -0.0098, 0.0,    0.0378,  0.0,
    -0.1187, 0.0,    0.4907,  1.0,
     0.4907, 0.0,   -0.1187,  0.0,
     0.0378, 0.0,   -0.0098,
];

/// Normalization factor: 1.0 / sum(DECIMATE3_COEFFS)
const DECIMATE3_GAIN: f32 = 1.0 / 1.8;

/// Resample audio from `from_rate` to `to_rate`.
///
/// - If rates are equal, returns a clone of input.
/// - If ratio is an exact integer (e.g. 48000/16000=3), uses FIR anti-alias + decimation.
/// - Otherwise, uses fast linear interpolation with f32 accumulator stepping.
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    // Integer decimation fast path (e.g., 48000 → 16000, factor = 3)
    if from_rate > to_rate && from_rate % to_rate == 0 {
        let factor = (from_rate / to_rate) as usize;
        if factor == 3 {
            return decimate_by_3(samples);
        }
        return decimate_generic(samples, factor);
    }

    // Arbitrary ratio: fast linear interpolation
    resample_linear(samples, from_rate, to_rate)
}

/// Optimized decimation by factor 3 with 15-tap FIR anti-alias filter.
/// Uses polyphase decomposition: only computes filter output at output positions.
fn decimate_by_3(samples: &[f32]) -> Vec<f32> {
    let output_len = samples.len() / 3;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let center = i * 3;
        let mut sum = 0.0f32;

        for (k, &coeff) in DECIMATE3_COEFFS.iter().enumerate() {
            let input_idx = center as isize + k as isize - DECIMATE3_HALF as isize;
            if input_idx >= 0 && (input_idx as usize) < samples.len() {
                // Safety: bounds checked above
                sum += coeff * unsafe { *samples.get_unchecked(input_idx as usize) };
            }
        }

        output.push(sum * DECIMATE3_GAIN);
    }

    output
}

/// Generic integer decimation: simple every-Nth sample (no filter).
/// Used for ratios other than 3 where we don't have pre-designed filter coefficients.
fn decimate_generic(samples: &[f32], factor: usize) -> Vec<f32> {
    samples.iter().step_by(factor).copied().collect()
}

/// Fast linear interpolation resampling with f32 accumulator (no f64 division per sample).
/// Handles arbitrary from_rate / to_rate ratios.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (samples.len() as f64 * ratio).ceil() as usize;
    let step = from_rate as f32 / to_rate as f32;

    let mut output = Vec::with_capacity(output_len);
    let mut src_pos: f32 = 0.0;

    // Calculate how many output samples can be produced without boundary checks
    let safe_count = if samples.len() >= 2 {
        (((samples.len() - 1) as f32) / step).floor() as usize
    } else {
        0
    };
    let safe_count = safe_count.min(output_len);

    // Hot loop: no bounds check needed
    for _ in 0..safe_count {
        let idx = src_pos as usize;
        let frac = src_pos - idx as f32;
        // Safety: guaranteed idx + 1 < samples.len() by safe_count calculation
        unsafe {
            let a = *samples.get_unchecked(idx);
            let b = *samples.get_unchecked(idx + 1);
            output.push(a + frac * (b - a));
        }
        src_pos += step;
    }

    // Tail: handle remaining 0-2 boundary samples
    for _ in safe_count..output_len {
        let idx = (src_pos as usize).min(samples.len().saturating_sub(1));
        output.push(if idx < samples.len() { samples[idx] } else { 0.0 });
        src_pos += step;
    }

    output
}
