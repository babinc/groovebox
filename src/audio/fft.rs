use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;

use super::types::SpectrumData;

const NUM_BINS: usize = 32;
const FFT_SIZE: usize = 2048;
const SAMPLE_RATE: f32 = 44100.0;

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
                // Send silent spectrum while inactive
                let _ = spectrum_tx.send(SpectrumData::default());
            }

            // Try to capture from PulseAudio monitor
            if let Err(_) = run_parec_capture(&spectrum_tx) {
                // Fallback: silent
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    });
}

/// Use `parec` to capture from PulseAudio monitor sink (actual playback audio)
fn run_parec_capture(spectrum_tx: &watch::Sender<SpectrumData>) -> Result<(), Box<dyn std::error::Error>> {
    use rustfft::{FftPlanner, num_complex::Complex};
    use std::io::Read;
    use std::process::{Command, Stdio};

    // Find the default sink's monitor source
    let sink_output = Command::new("pactl")
        .args(["get-default-sink"])
        .output()?;
    let default_sink = String::from_utf8_lossy(&sink_output.stdout).trim().to_string();

    if default_sink.is_empty() {
        return Err("No default PulseAudio sink found".into());
    }

    let monitor_source = format!("{default_sink}.monitor");

    // Spawn parec to capture raw PCM from the monitor source
    let mut child = Command::new("parec")
        .args([
            "--device", &monitor_source,
            "--format=float32le",
            "--channels=1",
            "--rate=44100",
            "--latency-msec=33",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to capture parec stdout")?;
    let mut reader = std::io::BufReader::new(stdout);

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut prev_bins = vec![0.0f32; NUM_BINS];
    let mut sample_buf: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut raw_buf = [0u8; 4 * 1024]; // Read in chunks (1024 float32 samples)

    loop {
        if !FFT_ACTIVE.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = spectrum_tx.send(SpectrumData::default());
            return Ok(());
        }

        let bytes_read = reader.read(&mut raw_buf)?;
        if bytes_read == 0 {
            break;
        }

        // Convert bytes to f32 samples
        let num_samples = bytes_read / 4;
        for i in 0..num_samples {
            let offset = i * 4;
            if offset + 4 <= bytes_read {
                let sample = f32::from_le_bytes([
                    raw_buf[offset],
                    raw_buf[offset + 1],
                    raw_buf[offset + 2],
                    raw_buf[offset + 3],
                ]);
                sample_buf.push(sample);
            }
        }

        // Keep buffer bounded
        if sample_buf.len() > FFT_SIZE * 4 {
            let drain_len = sample_buf.len() - FFT_SIZE;
            sample_buf.drain(..drain_len);
        }

        // Only process when we have enough samples
        if sample_buf.len() < FFT_SIZE {
            continue;
        }

        // Apply Hann window
        let samples = &sample_buf[sample_buf.len() - FFT_SIZE..];
        let mut fft_input: Vec<Complex<f32>> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
                Complex::new(s * window, 0.0)
            })
            .collect();

        fft.process(&mut fft_input);

        // Bin into log-scale frequency bands
        let mut bins = vec![0.0f32; NUM_BINS];
        let nyquist = FFT_SIZE / 2;

        for bin_idx in 0..NUM_BINS {
            let freq_low = 20.0 * (20000.0 / 20.0_f32).powf(bin_idx as f32 / NUM_BINS as f32);
            let freq_high = 20.0 * (20000.0 / 20.0_f32).powf((bin_idx + 1) as f32 / NUM_BINS as f32);

            let idx_low = (freq_low * FFT_SIZE as f32 / SAMPLE_RATE).round() as usize;
            let idx_high = (freq_high * FFT_SIZE as f32 / SAMPLE_RATE).round() as usize;

            let idx_low = idx_low.clamp(0, nyquist - 1);
            let idx_high = idx_high.clamp(idx_low + 1, nyquist);

            let mut sum = 0.0f32;
            let count = (idx_high - idx_low).max(1);
            for i in idx_low..idx_high {
                let mag = fft_input[i].norm();
                sum += mag;
            }
            bins[bin_idx] = sum / count as f32;
        }

        // Normalize to 0.0-1.0 range
        let max_val = bins.iter().cloned().fold(0.0f32, f32::max).max(0.001);
        for b in &mut bins {
            *b = (*b / max_val).min(1.0);
        }

        // Exponential decay smoothing
        for i in 0..NUM_BINS {
            prev_bins[i] = if bins[i] > prev_bins[i] {
                bins[i]
            } else {
                prev_bins[i] * 0.85 + bins[i] * 0.15
            };
        }

        let _ = spectrum_tx.send(SpectrumData { bins: prev_bins.clone() });
    }

    let _ = child.kill();
    Ok(())
}
