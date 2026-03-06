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
const FFT_SIZE: usize = 2048;

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
    use rustfft::{FftPlanner, num_complex::Complex};

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

    // FFT processing loop
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut prev_bins = [0.0f32; NUM_BINS];
    let mut rolling_max: f32 = 0.001;
    let mut fft_input = vec![Complex::new(0.0f32, 0.0); FFT_SIZE];
    let mut bins = [0.0f32; NUM_BINS];
    let mut local_samples = Vec::with_capacity(FFT_SIZE * 2);

    loop {
        if !FFT_ACTIVE.load(Ordering::Relaxed) {
            drop(stream);
            let _ = spectrum_tx.send(SpectrumData::default());
            return Ok(());
        }

        // Sleep to accumulate samples (~60 FPS processing rate)
        std::thread::sleep(std::time::Duration::from_millis(16));

        // Grab samples from the shared buffer
        if let Ok(mut buf) = sample_buf.lock() {
            local_samples.extend(buf.drain(..));
        }

        // Only keep the most recent FFT_SIZE samples to minimize latency
        if local_samples.len() > FFT_SIZE {
            let drain = local_samples.len() - FFT_SIZE;
            local_samples.drain(..drain);
        }

        if local_samples.len() < FFT_SIZE {
            continue;
        }

        // Apply Hann window
        let samples = &local_samples[local_samples.len() - FFT_SIZE..];
        for (i, &s) in samples.iter().enumerate() {
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            fft_input[i] = Complex::new(s * window, 0.0);
        }

        fft.process(&mut fft_input);

        // Bin into frequency bands using a sqrt-log scale that spreads
        // energy more evenly across the visual range.
        bins.fill(0.0);
        let nyquist = FFT_SIZE / 2;

        let f_min: f32 = 80.0;
        let f_max: f32 = 16000.0;

        for bin_idx in 0..NUM_BINS {
            let t0 = bin_idx as f32 / NUM_BINS as f32;
            let t1 = (bin_idx + 1) as f32 / NUM_BINS as f32;

            // True logarithmic scale: each octave gets equal visual width.
            // Maps t in [0,1] to freq in [f_min, f_max] via exponential interpolation.
            let log_min = f_min.ln();
            let log_max = f_max.ln();
            let freq_low = (log_min + (log_max - log_min) * t0).exp();
            let freq_high = (log_min + (log_max - log_min) * t1).exp();

            let idx_low = (freq_low * FFT_SIZE as f32 / sample_rate).round() as usize;
            let idx_high = (freq_high * FFT_SIZE as f32 / sample_rate).round() as usize;

            let idx_low = idx_low.clamp(0, nyquist - 1);
            let idx_high = idx_high.clamp(idx_low + 1, nyquist);

            let mut sum = 0.0f32;
            let count = (idx_high - idx_low).max(1);
            for i in idx_low..idx_high {
                let mag = fft_input[i].norm();
                sum += mag;
            }
            let avg = sum / count as f32;

            // A-weighting inspired perceptual curve.
            // Real spectrum analyzers do this to match human hearing.
            // Without it, bass dominates because it carries more energy
            // even though we don't perceive it as louder.
            let freq_center = (freq_low + freq_high) * 0.5;
            let a_weight = a_weight_gain(freq_center);
            bins[bin_idx] = avg * a_weight;
        }

        // Normalize against the frame's peak so the display uses full height
        let frame_max = bins.iter().cloned().fold(0.0f32, f32::max);
        rolling_max = if frame_max > rolling_max {
            rolling_max * 0.2 + frame_max * 0.8
        } else {
            rolling_max * 0.98 + frame_max * 0.02
        };
        rolling_max = rolling_max.max(0.001);
        for b in &mut bins {
            *b = (*b / rolling_max).min(1.0);
        }

        // Exponential smoothing — instant attack, fast decay
        for i in 0..NUM_BINS {
            prev_bins[i] = if bins[i] > prev_bins[i] {
                bins[i]
            } else {
                prev_bins[i] * 0.55 + bins[i] * 0.45
            };
        }

        let _ = spectrum_tx.send(SpectrumData { bins: prev_bins });
    }
}
