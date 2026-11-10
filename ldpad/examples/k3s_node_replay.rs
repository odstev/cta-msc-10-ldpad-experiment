//! Replay a recorded single-node "kubectl top nodes --no-headers" capture.
//!
//! Each sample consists of a date -Is line followed by one node-metrics line.
//! CPU is evaluated in millicores and memory in MiB with the existing LDPAD
//! AnomalyDetector (100-sample window, 3 sigma, 30-sample warm-up).
//!
//! The existing MetricSample API requires five numeric fields. Request rate,
//! error rate and network I/O were not collected; this two-metric pilot holds
//! those three internal fields fixed at zero solely to satisfy that API. They
//! are not observations, are omitted from output, and cannot trigger a
//! deviation while held constant. This is a replay, not live monitoring.
//!
//! Input is read from stdin; the CSV output is written to stdout. Diagnostics
//! are written to stderr. No attack labels or true/false-positive rates are
//! produced.

use std::io::{self, Read, Write};

use ldpad::anomaly::{AnomalyDetector, MetricKind, MetricSample};

const EXPECTED_SAMPLES: usize = 40;
const WARMUP_SAMPLES: usize = 30;

#[derive(Debug, PartialEq)]
struct Sample {
    captured_at: String,
    node: String,
    cpu_millicores: f64,
    memory_mib: f64,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn quantity(value: &str, suffixes: &[(&str, f64)], kind: &str) -> io::Result<f64> {
    for &(suffix, scale) in suffixes {
        if let Some(number) = value.strip_suffix(suffix) {
            let parsed: f64 = number
                .parse()
                .map_err(|_| invalid(format!("Invalid {kind} quantity: {value}")))?;
            let converted = parsed * scale;
            if parsed < 0.0 || !converted.is_finite() {
                return Err(invalid(format!("Invalid {kind} quantity: {value}")));
            }
            return Ok(converted);
        }
    }
    Err(invalid(format!("Unknown {kind} unit: {value}")))
}

fn cpu_millicores(value: &str) -> io::Result<f64> {
    quantity(value, &[("m", 1.0), ("u", 0.001), ("n", 0.000001), ("", 1000.0)], "CPU")
}

fn memory_mib(value: &str) -> io::Result<f64> {
    quantity(
        value,
        &[
            ("Ki", 1.0 / 1024.0),
            ("Mi", 1.0),
            ("Gi", 1024.0),
            ("Ti", 1024.0 * 1024.0),
            ("k", 1000.0 / 1048576.0),
            ("M", 1000000.0 / 1048576.0),
            ("G", 1000000000.0 / 1048576.0),
            ("", 1.0 / 1048576.0),
        ],
        "memory",
    )
}

fn parse_capture(input: &str) -> io::Result<Vec<Sample>> {
    let mut lines = input.lines().map(str::trim).filter(|line| !line.is_empty());
    let mut samples = Vec::new();
    let mut expected_node: Option<String> = None;

    while let Some(timestamp) = lines.next() {
        if !timestamp.contains('T') || timestamp.len() < 19 || timestamp.contains(',') {
            return Err(invalid(format!("Expected a date -Is timestamp, found: {timestamp}")));
        }
        let row = lines.next().ok_or_else(|| invalid("Missing node reading after timestamp"))?;
        let fields: Vec<_> = row.split_whitespace().collect();
        if fields.len() != 5 || fields[0].contains(',') {
            return Err(invalid(format!("Expected one five-column node reading, found: {row}")));
        }
        if let Some(ref name) = expected_node {
            if name.as_str() != fields[0] {
                return Err(invalid("Capture contains more than one node"));
            }
        } else {
            expected_node = Some(fields[0].to_owned());
        }
        samples.push(Sample {
            captured_at: timestamp.to_owned(),
            node: fields[0].to_owned(),
            cpu_millicores: cpu_millicores(fields[1])?,
            memory_mib: memory_mib(fields[3])?,
        });
    }
    if samples.is_empty() {
        return Err(invalid("Capture is empty"));
    }
    Ok(samples)
}

fn replay(samples: &[Sample], mut out: impl Write) -> io::Result<()> {
    let mut detector = AnomalyDetector::new(100, 3.0, WARMUP_SAMPLES);
    writeln!(out, "sample,captured_at,node,cpu_millicores,memory_mib,phase,cpu_alert,memory_alert,score")?;
    for (index, sample) in samples.iter().enumerate() {
        let metrics = MetricSample {
            cpu: sample.cpu_millicores,
            memory: sample.memory_mib,
            // Unavailable channels, held constant and never reported as observations.
            request_rate: 0.0,
            error_rate: 0.0,
            network_io: 0.0,
        };
        let result = detector.observe(&metrics);
        let cpu = result.triggered.iter().any(|(kind, _)| *kind == MetricKind::Cpu);
        let memory = result.triggered.iter().any(|(kind, _)| *kind == MetricKind::Memory);
        if result.triggered.iter().any(|(kind, _)| *kind != MetricKind::Cpu && *kind != MetricKind::Memory) {
            return Err(invalid("An unavailable metric unexpectedly triggered"));
        }
        let phase = if index < WARMUP_SAMPLES { "warmup" } else { "evaluation" };
        writeln!(
            out, "{},{},{},{:.6},{:.6},{},{},{},{:.6}",
            index + 1, sample.captured_at, sample.node, sample.cpu_millicores,
            sample.memory_mib, phase, u8::from(cpu), u8::from(memory), result.score
        )?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let samples = parse_capture(&input)?;
    if samples.len() != EXPECTED_SAMPLES {
        return Err(invalid(format!(
            "Expected {EXPECTED_SAMPLES} readings; found {}. Check the capture for errors or appended runs.",
            samples.len()
        )));
    }
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    replay(&samples, &mut output)?;
    output.flush()?;
    eprintln!(
        "Replayed {} recorded node samples: {} warm-up, {} evaluation. CPU and memory only; no attack labels.",
        samples.len(), WARMUP_SAMPLES, samples.len() - WARMUP_SAMPLES
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_kubernetes_quantities() {
        assert!((cpu_millicores("269m").unwrap() - 269.0).abs() < 1e-9);
        assert!((cpu_millicores("1").unwrap() - 1000.0).abs() < 1e-9);
        assert!((memory_mib("3973Mi").unwrap() - 3973.0).abs() < 1e-9);
        assert!((memory_mib("2Gi").unwrap() - 2048.0).abs() < 1e-9);
        assert!(cpu_millicores("bad").is_err());
    }

    #[test]
    fn parses_a_recorded_node_sample() {
        let input = "2026-11-08T05:06:54+03:00\nuser 269m 26% 3973Mi 67%\n";
        let rows = parse_capture(input).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].node, "user");
        assert_eq!(rows[0].cpu_millicores, 269.0);
        assert_eq!(rows[0].memory_mib, 3973.0);
        assert!(parse_capture("2026-11-08T05:06:54+03:00\n").is_err());
    }

    #[test]
    fn detects_a_cpu_spike_after_warmup_without_unused_alerts() {
        let mut rows = Vec::new();
        for i in 0..31 {
            rows.push(Sample {
                captured_at: format!("2026-11-08T05:00:{:02}+03:00", i),
                node: "user".to_owned(),
                cpu_millicores: if i == 30 { 5000.0 } else { 100.0 + (i % 3) as f64 },
                memory_mib: 2000.0 + (i % 3) as f64,
            });
        }
        let mut output = Vec::new();
        replay(&rows, &mut output).unwrap();
        let csv = String::from_utf8(output).unwrap();
        let last = csv.lines().last().unwrap();
        let fields: Vec<_> = last.split(',').collect();
        assert_eq!(fields[5], "evaluation");
        assert_eq!(fields[6], "1");
        assert_eq!(fields[7], "0");
    }
}
