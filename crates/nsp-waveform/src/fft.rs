//! FFT (Fast Fourier Transform) computation for frequency-domain analysis.
//!
//! Implements radix-2 Cooley-Tukey FFT, plus windowing functions (Hanning,
//! Blackman, Hamming) and magnitude/phase extraction. Used by the waveform
//! workspace to display real FFT and Bode plot data.
//!
//! # Usage
//!
//! ```rust,ignore
//! use nsp_waveform::fft::{fft_magnitude_db, fft_phase_deg, apply_window, WindowFunction};
//!
//! let samples: Vec<f64> = /* time-domain signal */;
//! let dt = 1e-9; // time step
//! let (freqs, magnitudes_db) = fft_magnitude_db(&samples, dt);
//! let (freqs, phases_deg) = fft_phase_deg(&samples, dt);
//! ```

/// Window function types for spectral leakage reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFunction {
    /// No windowing (rectangular).
    Rectangular,
    /// Hann (raised cosine) window — good general-purpose choice.
    Hanning,
    /// Blackman window — excellent side-lobe rejection.
    Blackman,
    /// Hamming window — improved Hanning variant.
    Hamming,
}

impl WindowFunction {
    /// Apply the window function to a slice of samples.
    pub fn apply(self, samples: &[f64]) -> Vec<f64> {
        let n = samples.len();
        samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let w = match self {
                    Self::Rectangular => 1.0,
                    Self::Hanning => {
                        0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos())
                    }
                    Self::Blackman => {
                        let ratio = i as f64 / n as f64;
                        0.42 - 0.5 * (2.0 * std::f64::consts::PI * ratio).cos()
                            + 0.08 * (4.0 * std::f64::consts::PI * ratio).cos()
                    }
                    Self::Hamming => {
                        0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()
                    }
                };
                s * w
            })
            .collect()
    }
}

/// A single frequency bin from FFT output.
#[derive(Debug, Clone)]
pub struct FftBin {
    /// Frequency in Hz.
    pub frequency: f64,
    /// Magnitude in linear scale.
    pub magnitude: f64,
    /// Phase in radians.
    pub phase: f64,
    /// Magnitude in dB (20 * log10).
    pub magnitude_db: f64,
    /// Phase in degrees.
    pub phase_deg: f64,
}

/// Compute in-place radix-2 Cooley-Tukey FFT.
///
/// On input, `real` and `imag` must have length that is a power of 2.
/// On output, they contain the DFT coefficients.
fn fft_in_place(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    debug_assert!(n.is_power_of_two());
    debug_assert_eq!(n, imag.len());

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Cooley-Tukey butterfly
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle_step = -2.0 * std::f64::consts::PI / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..half {
                let angle = angle_step * k as f64;
                let w_real = angle.cos();
                let w_imag = angle.sin();
                let t_real = w_real * real[start + k + half] - w_imag * imag[start + k + half];
                let t_imag = w_real * imag[start + k + half] + w_imag * real[start + k + half];
                let u_real = real[start + k];
                let u_imag = imag[start + k];
                real[start + k] = u_real + t_real;
                imag[start + k] = u_imag + t_imag;
                real[start + k + half] = u_real - t_real;
                imag[start + k + half] = u_imag - t_imag;
            }
        }
        len <<= 1;
    }
}

/// Round up to the next power of 2.
fn next_power_of_2(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    let mut p = 1usize;
    while p < n {
        p <<= 1;
    }
    p
}

/// Compute FFT magnitude in dB from time-domain samples.
///
/// Returns `(frequencies_hz, magnitudes_db)`. The input is zero-padded to the
/// next power of 2 if necessary.
pub fn fft_magnitude_db(samples: &[f64], dt: f64) -> (Vec<f64>, Vec<f64>) {
    let result = compute_fft_bins(samples, dt, WindowFunction::Hanning);
    let freqs: Vec<f64> = result.iter().map(|b| b.frequency).collect();
    let mags: Vec<f64> = result.iter().map(|b| b.magnitude_db).collect();
    (freqs, mags)
}

/// Compute FFT phase in degrees from time-domain samples.
///
/// Returns `(frequencies_hz, phases_deg)`.
pub fn fft_phase_deg(samples: &[f64], dt: f64) -> (Vec<f64>, Vec<f64>) {
    let result = compute_fft_bins(samples, dt, WindowFunction::Hanning);
    let freqs: Vec<f64> = result.iter().map(|b| b.frequency).collect();
    let phases: Vec<f64> = result.iter().map(|b| b.phase_deg).collect();
    (freqs, phases)
}

/// Compute full FFT result with configurable window.
///
/// Returns `Vec<FftBin>` containing frequency, magnitude, and phase for each
/// bin. Only the positive-frequency half is returned (Nyquist frequency excluded).
pub fn compute_fft_bins(samples: &[f64], dt: f64, window: WindowFunction) -> Vec<FftBin> {
    if samples.is_empty() || dt <= 0.0 {
        return Vec::new();
    }

    let windowed = window.apply(samples);
    let n_padded = next_power_of_2(windowed.len());
    let fs = 1.0 / dt;

    let mut real: Vec<f64> = windowed;
    real.resize(n_padded, 0.0);
    let mut imag: Vec<f64> = vec![0.0; n_padded];

    fft_in_place(&mut real, &mut imag);

    // Only return positive frequencies (first half + DC)
    let half = n_padded / 2;
    let mut bins = Vec::with_capacity(half + 1);

    for k in 0..=half {
        let freq = k as f64 * fs / n_padded as f64;
        let re = real[k];
        let im = imag[k];
        let magnitude = (re * re + im * im).sqrt();
        // Normalize: FFT magnitude scaled by N, and 2x for single-sided spectrum (except DC)
        let norm_magnitude = if k == 0 || k == half {
            magnitude / n_padded as f64
        } else {
            2.0 * magnitude / n_padded as f64
        };
        let phase = im.atan2(re);
        let magnitude_db = if norm_magnitude > 1e-15 {
            20.0 * norm_magnitude.log10()
        } else {
            -300.0 // effectively silence
        };

        bins.push(FftBin {
            frequency: freq,
            magnitude: norm_magnitude,
            phase,
            magnitude_db,
            phase_deg: phase * 180.0 / std::f64::consts::PI,
        });
    }

    bins
}

/// Compute a Bode plot (magnitude + phase) from time-domain transient data.
///
/// This is a convenience wrapper around `compute_fft_bins` that returns the
/// data in the format expected by the Bode plot UI.
pub fn compute_bode(
    voltage_signal: &[f64],
    current_signal: Option<&[f64]>,
    dt: f64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    // For impedance Bode: V/I. For voltage Bode: just V.
    if let Some(current) = current_signal {
        // Compute impedance = V/I in frequency domain
        let v_bins = compute_fft_bins(voltage_signal, dt, WindowFunction::Hanning);
        let i_bins = compute_fft_bins(current, dt, WindowFunction::Hanning);

        let len = v_bins.len().min(i_bins.len());
        let mut freqs = Vec::with_capacity(len);
        let mut magnitudes_db = Vec::with_capacity(len);
        let mut phases_deg = Vec::with_capacity(len);

        for k in 0..len {
            let freq = v_bins[k].frequency;
            let i_mag = i_bins[k].magnitude;
            let z_mag = if i_mag > 1e-15 {
                v_bins[k].magnitude / i_mag
            } else {
                0.0
            };
            let z_phase = v_bins[k].phase - i_bins[k].phase;

            freqs.push(freq);
            magnitudes_db.push(if z_mag > 1e-15 {
                20.0 * z_mag.log10()
            } else {
                -300.0
            });
            phases_deg.push(z_phase * 180.0 / std::f64::consts::PI);
        }

        (freqs, magnitudes_db, phases_deg)
    } else {
        // Simple voltage magnitude Bode
        let bins = compute_fft_bins(voltage_signal, dt, WindowFunction::Hanning);
        let freqs: Vec<f64> = bins.iter().map(|b| b.frequency).collect();
        let mags: Vec<f64> = bins.iter().map(|b| b.magnitude_db).collect();
        let phases: Vec<f64> = bins.iter().map(|b| b.phase_deg).collect();
        (freqs, mags, phases)
    }
}

/// Export FFT data to CSV format for external analysis.
pub fn fft_to_csv(bins: &[FftBin]) -> String {
    let mut output = String::from("frequency_hz,magnitude,magnitude_db,phase_rad,phase_deg\n");
    for bin in bins {
        output.push_str(&format!(
            "{},{},{},{},{}\n",
            bin.frequency, bin.magnitude, bin.magnitude_db, bin.phase, bin.phase_deg
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: generate a sine wave.
    fn sine_wave(freq: f64, dt: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 * dt).sin())
            .collect()
    }

    #[test]
    fn fft_detects_sine_frequency() {
        let freq = 1000.0; // 1 kHz
        let fs = 44100.0;
        let dt = 1.0 / fs;
        let n = 1024; // power of 2
        let samples = sine_wave(freq, dt, n);

        let bins = compute_fft_bins(&samples, dt, WindowFunction::Hanning);

        // Find the bin with maximum magnitude (should be near 1 kHz)
        let peak = bins
            .iter()
            .max_by(|a, b| a.magnitude.partial_cmp(&b.magnitude).unwrap())
            .unwrap();
        let freq_error = (peak.frequency - freq).abs();
        assert!(
            freq_error < 100.0,
            "Peak frequency {} is too far from expected {} (error: {} Hz)",
            peak.frequency,
            freq,
            freq_error
        );
    }

    #[test]
    fn fft_magnitude_db_output() {
        let samples = sine_wave(440.0, 1.0 / 44100.0, 1024);
        let (freqs, mags) = fft_magnitude_db(&samples, 1.0 / 44100.0);
        assert_eq!(freqs.len(), mags.len());
        assert!(!freqs.is_empty());
        // DC should be near silence
        assert!(mags[0] < -50.0);
    }

    #[test]
    fn fft_phase_output() {
        let samples = sine_wave(440.0, 1.0 / 44100.0, 1024);
        let (freqs, phases) = fft_phase_deg(&samples, 1.0 / 44100.0);
        assert_eq!(freqs.len(), phases.len());
        // All phase values should be in [-180, 180]
        for p in &phases {
            assert!(p.abs() <= 180.0 + 1e-6, "Phase {} out of range", p);
        }
    }

    #[test]
    fn window_functions_differ() {
        let samples = vec![1.0, 2.0, 3.0, 4.0];
        let rect = WindowFunction::Rectangular.apply(&samples);
        let hann = WindowFunction::Hanning.apply(&samples);
        let black = WindowFunction::Blackman.apply(&samples);
        assert_eq!(rect, samples); // Rectangular is identity
        assert_ne!(rect, hann);
        assert_ne!(hann, black);
    }

    #[test]
    fn fft_zero_input() {
        let bins = compute_fft_bins(&[], 1e-9, WindowFunction::Hanning);
        assert!(bins.is_empty());
    }

    #[test]
    fn fft_to_csv_output() {
        let bins = vec![
            FftBin {
                frequency: 0.0,
                magnitude: 1.0,
                phase: 0.0,
                magnitude_db: 0.0,
                phase_deg: 0.0,
            },
            FftBin {
                frequency: 1000.0,
                magnitude: 0.5,
                phase: 0.785,
                magnitude_db: -6.0,
                phase_deg: 45.0,
            },
        ];
        let csv = fft_to_csv(&bins);
        assert!(csv.contains("frequency_hz"));
        assert!(csv.contains("1000"));
    }

    #[test]
    fn bode_without_current() {
        let samples = sine_wave(100.0, 1.0 / 10000.0, 1024);
        let (freqs, mags, phases) = compute_bode(&samples, None, 1.0 / 10000.0);
        assert_eq!(freqs.len(), mags.len());
        assert_eq!(freqs.len(), phases.len());
    }

    #[test]
    fn bode_with_current_impedance() {
        let dt = 1.0 / 10000.0;
        let n = 1024;
        let voltage = sine_wave(100.0, dt, n);
        let current: Vec<f64> = voltage.iter().map(|v| v / 1000.0).collect(); // 1k resistor
        let (freqs, mags, _phases) = compute_bode(&voltage, Some(&current), dt);
        // Impedance of 1k resistor should be ~60 dB
        if let Some(&peak_mag) = freqs
            .iter()
            .zip(mags.iter())
            .filter(|(f, _)| **f > 50.0 && **f < 200.0)
            .map(|(_, m)| m)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
        {
            assert!(
                (peak_mag - 60.0).abs() < 10.0,
                "Expected ~60 dB, got {}",
                peak_mag
            );
        }
    }

    #[test]
    fn next_power_of_2_values() {
        assert_eq!(next_power_of_2(0), 1);
        assert_eq!(next_power_of_2(1), 1);
        assert_eq!(next_power_of_2(2), 2);
        assert_eq!(next_power_of_2(3), 4);
        assert_eq!(next_power_of_2(7), 8);
        assert_eq!(next_power_of_2(1000), 1024);
    }
}
