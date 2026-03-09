use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use super::types::SpectrumData;

#[cfg(debug_assertions)]
fn log_fft(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/tmp/groovebox.log")
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(f, "[{:.3}] FFT: {msg}", now.as_secs_f64());
    }
}

#[cfg(not(debug_assertions))]
fn log_fft(_msg: &str) {}

const NUM_BINS: usize = 64;
const FFT_SIZE: usize = 8192;

/// Flag to control whether FFT is active (only when audio is playing)
static FFT_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_fft_active(active: bool) {
    FFT_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn spawn_fft_task(spectrum_tx: watch::Sender<SpectrumData>) {
    std::thread::spawn(move || {
        loop {
            // Wait until playback is active
            while !FFT_ACTIVE.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = spectrum_tx.send(SpectrumData::default());
            }

            // Try to capture system audio via cpal loopback
            if run_cpal_capture(&spectrum_tx).is_err() {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    });
}

/// Attempt at simplified A-weighting gain for perceptual loudness correction.
/// Based on the IEC 61672 standard curve, approximated for display purposes.
/// Returns a multiplier: <1 for bass attenuation, ~1 at 1-4kHz, slight rolloff above.
fn a_weight_gain(freq: f32) -> f32 {
    // Attempt at simplified A-weighting using the standard formula shape.
    // A(f) is roughly: very low at 20Hz, rises steeply to ~1kHz,
    // flat 1-6kHz, gentle rolloff above.
    let f2 = freq * freq;
    let numerator = 12194.0f32.powi(2) * f2 * f2;
    let denominator = (f2 + 20.6f32.powi(2))
        * ((f2 + 107.7f32.powi(2)) * (f2 + 737.9f32.powi(2))).sqrt()
        * (f2 + 12194.0f32.powi(2));
    if denominator < 1e-10 {
        return 0.0;
    }
    // Raw A-weight value (not in dB), normalized so 1kHz ≈ 1.0
    let raw = numerator / denominator;
    let at_1k = {
        let f2 = 1000.0f32 * 1000.0;
        let n = 12194.0f32.powi(2) * f2 * f2;
        let d = (f2 + 20.6f32.powi(2))
            * ((f2 + 107.7f32.powi(2)) * (f2 + 737.9f32.powi(2))).sqrt()
            * (f2 + 12194.0f32.powi(2));
        n / d
    };
    (raw / at_1k).max(0.0)
}

/// Capture system audio output using cpal's loopback/monitor capability.
/// On Linux (PulseAudio/PipeWire): opens the default output device as input (monitor source).
/// On macOS: uses Core Audio loopback (cpal git main, PR #1003).
fn run_cpal_capture(spectrum_tx: &watch::Sender<SpectrumData>) -> Result<(), Box<dyn std::error::Error>> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    // Prefer PipeWire host (exposes monitor sources), fall back to default
    let host = cpal::available_hosts().into_iter()
        .find(|id| id.name().contains("PipeWire"))
        .and_then(|id| cpal::host_from_id(id).ok())
        .unwrap_or_else(cpal::default_host);

    log_fft(&format!("HOST: {:?}", host.id().name()));

    // Log all available input devices
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let name = d.description().map(|desc| desc.name().to_string()).unwrap_or_default();
            log_fft(&format!("INPUT DEVICE: '{name}'"));
        }
    }

    // Find monitor source for capturing system audio output.
    // PulseAudio: look for "*.monitor" input devices.
    // PipeWire: look for "sink_default" (loopback of default sink) or "*.monitor".
    // Fallback: default output device (macOS Core Audio loopback).
    let device = host.input_devices()?
        .find(|d| {
            let name = d.description().map(|desc| desc.name().to_string()).unwrap_or_default();
            name.contains(".monitor") || name == "sink_default"
        })
        .or_else(|| host.default_output_device())
        .ok_or("No monitor or output audio device found")?;

    let device_name = device.description().map(|desc| desc.name().to_string()).unwrap_or_default();
    log_fft(&format!("SELECTED: '{device_name}'"));

    // Use input config for monitor sources, output config for loopback on output devices
    let supported_config = device.default_input_config()
        .or_else(|_| device.default_output_config())?;
    let channels = supported_config.channels() as usize;
    let sample_rate = supported_config.sample_rate() as f32;
    let sample_format = supported_config.sample_format();

    let stream_config: cpal::StreamConfig = supported_config.into();

    log_fft(&format!(
        "CPAL: rate={} ch={} fmt={:?}",
        sample_rate, channels, sample_format
    ));

    // Shared ring buffer for audio samples
    let sample_buf = Arc::new(Mutex::new(Vec::<f32>::with_capacity(FFT_SIZE * 4)));

    let sample_buf_writer = Arc::clone(&sample_buf);
    let err_fn = |err: cpal::StreamError| {
        let _ = err;
    };

    // Build an input stream on the output device (loopback capture).
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = sample_buf_writer.try_lock() {
                        // Mix down to mono
                        for chunk in data.chunks(channels) {
                            let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                        // Keep bounded
                        if buf.len() > FFT_SIZE * 2 {
                            let drain = buf.len() - FFT_SIZE;
                            buf.drain(..drain);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = sample_buf_writer.try_lock() {
                        for chunk in data.chunks(channels) {
                            let mono: f32 = chunk.iter()
                                .map(|&s| s as f32 / i16::MAX as f32)
                                .sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                        if buf.len() > FFT_SIZE * 2 {
                            let drain = buf.len() - FFT_SIZE;
                            buf.drain(..drain);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        format => return Err(format!("Unsupported sample format: {format:?}").into()),
    };

    stream.play()?;

    let mut proc = FftProcessor::new(sample_rate);

    loop {
        if !FFT_ACTIVE.load(Ordering::Relaxed) {
            drop(stream);
            let _ = spectrum_tx.send(SpectrumData::default());
            return Ok(());
        }

        std::thread::sleep(std::time::Duration::from_millis(16));

        // Drain shared buffer into processor
        if let Ok(mut buf) = sample_buf.lock() {
            proc.push_samples(buf.drain(..));
        }

        if let Some(spectrum) = proc.process() {
            let _ = spectrum_tx.send(spectrum);
        }
    }
}

const F_MIN: f32 = 80.0;
const F_MAX: f32 = 16000.0;

/// Encapsulates the entire FFT processing pipeline from raw mono samples to SpectrumData.
/// Extracted so it can be tested without audio hardware.
struct FftProcessor {
    sample_rate: f32,
    planner_fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    fft_input: Vec<rustfft::num_complex::Complex<f32>>,
    bins: [f32; NUM_BINS],
    prev_bins: [f32; NUM_BINS],
    rolling_max: f32,
    local_samples: Vec<f32>,
}

impl FftProcessor {
    fn new(sample_rate: f32) -> Self {
        use rustfft::{FftPlanner, num_complex::Complex};
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        Self {
            sample_rate,
            planner_fft: fft,
            fft_input: vec![Complex::new(0.0f32, 0.0); FFT_SIZE],
            bins: [0.0f32; NUM_BINS],
            prev_bins: [0.0f32; NUM_BINS],
            rolling_max: 0.001,
            local_samples: Vec::with_capacity(FFT_SIZE * 2),
        }
    }

    /// Add mono samples to the internal buffer.
    fn push_samples(&mut self, samples: impl Iterator<Item = f32>) {
        self.local_samples.extend(samples);
        // Only keep the most recent FFT_SIZE samples to minimize latency
        if self.local_samples.len() > FFT_SIZE {
            let drain = self.local_samples.len() - FFT_SIZE;
            self.local_samples.drain(..drain);
        }
    }

    /// Run one frame of FFT processing. Returns None if not enough samples yet.
    fn process(&mut self) -> Option<SpectrumData> {
        use rustfft::num_complex::Complex;

        if self.local_samples.len() < FFT_SIZE {
            return None;
        }

        // Apply Hann window
        let samples = &self.local_samples[self.local_samples.len() - FFT_SIZE..];
        for (i, &s) in samples.iter().enumerate() {
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            self.fft_input[i] = Complex::new(s * window, 0.0);
        }

        self.planner_fft.process(&mut self.fft_input);

        // Bin into frequency bands using logarithmic scale
        self.bins.fill(0.0);
        let nyquist = FFT_SIZE / 2;
        let log_min = F_MIN.ln();
        let log_max = F_MAX.ln();

        for bin_idx in 0..NUM_BINS {
            let t0 = bin_idx as f32 / NUM_BINS as f32;
            let t1 = (bin_idx + 1) as f32 / NUM_BINS as f32;

            // True logarithmic scale: each octave gets equal visual width.
            let freq_low = (log_min + (log_max - log_min) * t0).exp();
            let freq_high = (log_min + (log_max - log_min) * t1).exp();

            let idx_low = (freq_low * FFT_SIZE as f32 / self.sample_rate).round() as usize;
            let idx_high = (freq_high * FFT_SIZE as f32 / self.sample_rate).round() as usize;
            let idx_low = idx_low.clamp(0, nyquist - 1);
            let idx_high = idx_high.clamp(idx_low + 1, nyquist);

            let mut sum = 0.0f32;
            let count = (idx_high - idx_low).max(1);
            for i in idx_low..idx_high {
                sum += self.fft_input[i].norm();
            }
            let avg = sum / count as f32;

            // A-weighting perceptual curve to match human hearing
            let freq_center = (freq_low + freq_high) * 0.5;
            self.bins[bin_idx] = avg * a_weight_gain(freq_center);
        }

        // Normalize against rolling peak
        let frame_max = self.bins.iter().cloned().fold(0.0f32, f32::max);
        self.rolling_max = if frame_max > self.rolling_max {
            self.rolling_max * 0.2 + frame_max * 0.8
        } else {
            self.rolling_max * 0.98 + frame_max * 0.02
        };
        self.rolling_max = self.rolling_max.max(0.001);
        for b in &mut self.bins {
            *b = (*b / self.rolling_max).min(1.0);
        }

        // Exponential smoothing — instant attack, fast decay
        for i in 0..NUM_BINS {
            self.prev_bins[i] = if self.bins[i] > self.prev_bins[i] {
                self.bins[i]
            } else {
                self.prev_bins[i] * 0.55 + self.bins[i] * 0.45
            };
        }

        Some(SpectrumData { bins: self.prev_bins })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48000.0;
    const CHANNELS: usize = 2; // stereo, like real audio

    /// Given a frequency, return the expected visual bin index (0..NUM_BINS).
    fn expected_bin(freq: f32) -> usize {
        let log_min = F_MIN.ln();
        let log_max = F_MAX.ln();
        let t = (freq.ln() - log_min) / (log_max - log_min);
        (t * NUM_BINS as f32).floor().clamp(0.0, (NUM_BINS - 1) as f32) as usize
    }

    /// Generate stereo interleaved samples for a sine wave at the given frequency.
    /// This simulates what cpal delivers from the audio device.
    fn generate_stereo_samples(freq: f32, num_frames: usize) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_frames * CHANNELS);
        for i in 0..num_frames {
            let sample = (2.0 * std::f32::consts::PI * freq * i as f32 / SAMPLE_RATE).sin();
            // Same signal on both channels (mono content in stereo)
            for _ in 0..CHANNELS {
                samples.push(sample);
            }
        }
        samples
    }

    /// Simulate the cpal audio callback: mix stereo to mono and push to shared buffer.
    /// This is the same logic as the F32 input stream handler in production.
    fn push_stereo_to_buffer(buf: &mut Vec<f32>, stereo_data: &[f32]) {
        for chunk in stereo_data.chunks(CHANNELS) {
            let mono: f32 = chunk.iter().sum::<f32>() / CHANNELS as f32;
            buf.push(mono);
        }
        if buf.len() > FFT_SIZE * 2 {
            let drain = buf.len() - FFT_SIZE;
            buf.drain(..drain);
        }
    }

    /// Run the full pipeline for a single frequency: generate stereo audio →
    /// cpal callback simulation → shared buffer → FftProcessor → SpectrumData.
    /// Returns the final SpectrumData after several processing frames to let
    /// the rolling_max stabilize.
    fn full_pipeline(freq: f32) -> SpectrumData {
        let mut shared_buf = Vec::new();
        let mut proc = FftProcessor::new(SAMPLE_RATE);

        // Generate enough audio for multiple processing frames (~200ms)
        let stereo = generate_stereo_samples(freq, SAMPLE_RATE as usize / 5);

        // Feed in chunks (simulating cpal callback delivering ~1024 frames at a time)
        let chunk_size = 1024 * CHANNELS;
        let mut result = SpectrumData::default();

        for chunk in stereo.chunks(chunk_size) {
            push_stereo_to_buffer(&mut shared_buf, chunk);

            // Drain shared buffer into processor (simulating the main loop)
            proc.push_samples(shared_buf.drain(..));

            if let Some(spectrum) = proc.process() {
                result = spectrum;
            }
        }
        result
    }

    fn peak_bin(spectrum: &SpectrumData) -> usize {
        spectrum.bins.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0
    }

    /// Calculate the center frequency for a given visual bar index.
    fn bar_center_freq(bar: usize) -> f32 {
        let log_min = F_MIN.ln();
        let log_max = F_MAX.ln();
        let t = (bar as f32 + 0.5) / NUM_BINS as f32;
        (log_min + (log_max - log_min) * t).exp()
    }

    #[test]
    fn test_each_bar_responds_to_its_frequency() {
        // For every visual bar on the EQ, generate a tone at that bar's center
        // frequency and verify ONLY that bar (±1 neighbor) lights up.
        let mut all_passed = true;

        for bar in 0..NUM_BINS {
            let freq = bar_center_freq(bar);
            let spectrum = full_pipeline(freq);
            let actual_peak = peak_bin(&spectrum);
            let diff = (actual_peak as i32 - bar as i32).unsigned_abs();

            // The peak should land on this bar or an immediate neighbor
            if diff > 1 {
                eprintln!(
                    "FAIL: bar {bar} ({freq:.0}Hz) — peak landed on bar {actual_peak} (off by {diff})"
                );
                all_passed = false;
                continue;
            }

            // Verify distant bars are quiet: bars more than 4 away should have
            // less than 20% of the peak bar's energy
            let peak_val = spectrum.bins[actual_peak];
            if peak_val < 0.01 {
                // A-weighting may suppress extreme ends; skip isolation check
                eprintln!(
                    "  OK: bar {bar} ({freq:.0}Hz) — suppressed by A-weight (val={peak_val:.4})"
                );
                continue;
            }

            let mut isolation_ok = true;
            for other in 0..NUM_BINS {
                let dist = (other as i32 - bar as i32).unsigned_abs();
                if dist <= 3 { continue; }
                let ratio = spectrum.bins[other] / peak_val;
                if ratio > 0.20 {
                    eprintln!(
                        "FAIL: bar {bar} ({freq:.0}Hz) — bar {other} has {:.0}% of peak energy (should be <20%)",
                        ratio * 100.0
                    );
                    isolation_ok = false;
                    break;
                }
            }

            if isolation_ok {
                eprintln!(
                    "  OK: bar {bar} ({freq:.0}Hz) — peak at bar {actual_peak}, isolated"
                );
            } else {
                all_passed = false;
            }
        }
        assert!(all_passed, "Some bars did not correctly isolate their frequency");
    }

    #[test]
    fn test_full_pipeline_silence_produces_zero() {
        let mut proc = FftProcessor::new(SAMPLE_RATE);
        // Feed silence (all zeros)
        proc.push_samples(vec![0.0f32; FFT_SIZE].into_iter());
        let spectrum = proc.process().unwrap();
        let max = spectrum.bins.iter().cloned().fold(0.0f32, f32::max);
        assert!(max < 0.01, "Silence should produce near-zero bins, got max={max}");
    }

    #[test]
    fn test_full_pipeline_stereo_mixdown() {
        // Verify stereo→mono mixdown works correctly
        let mut buf = Vec::new();
        // Left=1.0, Right=-1.0 should cancel to 0.0
        let stereo_cancel = vec![1.0f32, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        push_stereo_to_buffer(&mut buf, &stereo_cancel);
        assert!(buf.iter().all(|&s| s.abs() < 1e-6), "L/R cancel should produce silence");

        // Left=0.5, Right=0.5 should produce 0.5
        buf.clear();
        let stereo_same = vec![0.5f32, 0.5, 0.5, 0.5];
        push_stereo_to_buffer(&mut buf, &stereo_same);
        assert!((buf[0] - 0.5).abs() < 1e-6, "Equal L/R should average to same value");
    }

    #[test]
    fn test_full_pipeline_output_range() {
        // All output bins should be in [0.0, 1.0] after normalization
        let spectrum = full_pipeline(1000.0);
        for (i, &val) in spectrum.bins.iter().enumerate() {
            assert!(val >= 0.0 && val <= 1.0, "Bin {i} out of range: {val}");
        }
    }

    #[test]
    fn test_full_pipeline_frequency_ordering() {
        // A low tone should peak in a lower bin than a high tone
        let low = full_pipeline(200.0);
        let high = full_pipeline(8000.0);
        let low_peak = peak_bin(&low);
        let high_peak = peak_bin(&high);
        assert!(
            low_peak < high_peak,
            "200Hz peak (bin {low_peak}) should be left of 8kHz peak (bin {high_peak})"
        );
    }

    #[test]
    fn test_a_weighting_shape() {
        let w100 = a_weight_gain(100.0);
        let w1k = a_weight_gain(1000.0);
        let w4k = a_weight_gain(4000.0);
        let w10k = a_weight_gain(10000.0);

        assert!(w100 < 0.15, "100Hz should be heavily attenuated, got {w100}");
        assert!((w1k - 1.0).abs() < 0.01, "1kHz should be ~1.0, got {w1k}");
        assert!(w4k > 0.5, "4kHz should be strong, got {w4k}");
        assert!(w10k < w4k, "10kHz should roll off vs 4kHz");
    }

    #[test]
    fn test_full_pipeline_buffer_bounding() {
        // Shared buffer should not grow unbounded
        let mut buf = Vec::new();
        let huge = generate_stereo_samples(440.0, FFT_SIZE * 10);
        push_stereo_to_buffer(&mut buf, &huge);
        assert!(buf.len() <= FFT_SIZE, "Buffer should be bounded to FFT_SIZE, got {}", buf.len());
    }
}
