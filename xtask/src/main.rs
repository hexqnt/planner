use std::{
    env,
    error::Error,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const HELP: &str = "Usage:
  cargo xtask scroll [run|record|stat|all] [output-directory]
  cargo xtask startup [run|record|stat|all] [output-directory]
  cargo xtask resize [run|record|stat|all] [output-directory]
  cargo xtask interaction [run|record|stat|all] [output-directory]
  cargo xtask compare BASELINE.json CURRENT.json

Default mode: run, results in target/profiles/<scenario>-<timestamp>.
Environment: planner_PROFILE_CYCLES (scroll: 20, startup/resize: 100, interaction: 5), planner_PROFILE_EVENTS (1000),
             PROFILE_EVENT (cycles:u), PROFILE_FREQ (499),
             PROFILE_STAT_EVENTS (task-clock,cycles,instructions,branches,branch-misses).
record/stat/all require Linux perf and access to the selected counters.";

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result {
    let mut args = env::args_os().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    match command.to_str() {
        Some(name @ ("scroll" | "startup" | "resize" | "interaction")) => {
            let scenario = match name {
                "startup" => Scenario::Startup,
                "resize" => Scenario::Resize,
                "interaction" => Scenario::Interaction,
                _ => Scenario::Scroll,
            };
            let mode = Mode::parse(args.next().as_deref())?;
            let directory = args.next().map(PathBuf::from);
            if args.next().is_some() {
                return Err(HELP.into());
            }
            profile(scenario, mode, directory)
        }
        Some("compare") => {
            let baseline = args.next().ok_or(HELP)?;
            let current = args.next().ok_or(HELP)?;
            if args.next().is_some() {
                return Err(HELP.into());
            }
            compare(Path::new(&baseline), Path::new(&current))
        }
        Some("help" | "--help" | "-h") => {
            println!("{HELP}");
            Ok(())
        }
        _ => Err(HELP.into()),
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    Scroll,
    Startup,
    Resize,
    Interaction,
}

impl Scenario {
    const fn name(self) -> &'static str {
        match self {
            Self::Scroll => "scroll",
            Self::Startup => "startup",
            Self::Resize => "resize",
            Self::Interaction => "interaction",
        }
    }

    const fn test(self) -> &'static str {
        match self {
            Self::Scroll => "profile_scroll",
            Self::Startup => "startup::profile_startup",
            Self::Resize => "resize::profile_resize",
            Self::Interaction => "interaction::profile_interaction",
        }
    }

    const fn frames_per_cycle(self) -> u32 {
        match self {
            Self::Scroll => 240,
            Self::Startup => 1,
            Self::Resize => 12,
            Self::Interaction => 2710,
        }
    }

    const fn cycles(self) -> u32 {
        match self {
            Self::Scroll => 20,
            Self::Startup | Self::Resize => 100,
            Self::Interaction => 5,
        }
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Run,
    Record,
    Stat,
    All,
}

impl Mode {
    fn parse(value: Option<&std::ffi::OsStr>) -> Result<Self> {
        let Some(value) = value else {
            return Ok(Self::Run);
        };
        match value.to_str() {
            Some("run") => Ok(Self::Run),
            Some("record") => Ok(Self::Record),
            Some("stat") => Ok(Self::Stat),
            Some("all") => Ok(Self::All),
            _ => Err(HELP.into()),
        }
    }
}

fn checked(command: &mut Command) -> Result {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("Command failed ({status}): {command:?}").into());
    }
    Ok(())
}

fn capture(command: &mut Command) -> Result<String> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "Command failed: {command:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn positive_setting(name: &str, default: u32) -> Result<u32> {
    match env::var(name) {
        Ok(value) => value
            .parse::<std::num::NonZeroU32>()
            .map(std::num::NonZeroU32::get)
            .map_err(|_| format!("{name} must be a positive integer").into()),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error.into()),
    }
}

fn build(root: &Path) -> Result<PathBuf> {
    let mut flags = env::var_os("CARGO_ENCODED_RUSTFLAGS").unwrap_or_else(|| {
        env::var("RUSTFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("\x1f")
            .into()
    });
    if !flags.is_empty() {
        flags.push("\x1f");
    }
    flags.push("-C\x1fforce-frame-pointers=yes");
    eprintln!("Building optimized profiling workload with symbols and frame pointers...");
    let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(root)
        .args([
            "test",
            "--locked",
            "--profile",
            "profiling",
            "--package",
            "planner-profiling",
            "--test",
            "workloads",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .env("CARGO_ENCODED_RUSTFLAGS", flags)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err("Unable to build profiling workload".into());
    }
    for line in String::from_utf8(output.stdout)?.lines() {
        let message: Value = serde_json::from_str(line)?;
        if message["reason"] == "compiler-artifact"
            && message["target"]["name"] == "workloads"
            && message["profile"]["test"] == true
            && let Some(path) = message["executable"].as_str()
        {
            return Ok(PathBuf::from(path));
        }
    }
    Err("Cargo did not report the profiling test executable".into())
}

fn workload(command: &mut Command, scenario: Scenario, metrics: &Path, cycles: u32, events: u32) {
    command.args([
        scenario.test(),
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]);
    command
        .env("planner_PROFILE_OUTPUT", metrics)
        .env("planner_PROFILE_CYCLES", cycles.to_string())
        .env("planner_PROFILE_EVENTS", events.to_string());
}

fn output_directory(
    root: &Path,
    scenario: Scenario,
    directory: Option<PathBuf>,
) -> Result<(PathBuf, u128)> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let directory = directory
        .unwrap_or_else(|| root.join(format!("target/profiles/{}-{timestamp}", scenario.name())));
    // Не перезаписываем исходные измерения при повторном запуске команды.
    if directory.exists() {
        return Err(format!("Output directory already exists: {}", directory.display()).into());
    }
    Ok((directory, timestamp))
}

fn create_output_directory(directory: &Path) -> Result<PathBuf> {
    if let Some(parent) = directory.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(directory)?;
    Ok(directory.canonicalize()?)
}

fn profile(scenario: Scenario, mode: Mode, directory: Option<PathBuf>) -> Result {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing workspace root")?;
    let cycles = positive_setting("planner_PROFILE_CYCLES", scenario.cycles())?;
    cycles
        .checked_mul(scenario.frames_per_cycle())
        .ok_or("Frame count overflow")?;
    let events = positive_setting("planner_PROFILE_EVENTS", 1000)?;
    let frequency = positive_setting("PROFILE_FREQ", 499)?;
    let (directory, timestamp) = output_directory(root, scenario, directory)?;
    if !matches!(mode, Mode::Run) {
        checked(Command::new("perf").arg("--version"))?;
    }
    let binary = build(root)?;
    let directory = create_output_directory(&directory)?;
    let profile_event = env::var("PROFILE_EVENT").unwrap_or_else(|_| "cycles:u".into());
    let stat_events = env::var("PROFILE_STAT_EVENTS")
        .unwrap_or_else(|_| "task-clock,cycles,instructions,branches,branch-misses".into());
    let metadata = json!({
        "timestamp_unix_ms": timestamp,
        "git_revision": capture(Command::new("git").current_dir(root).args(["rev-parse", "HEAD"]))?.trim(),
        "git_status": capture(Command::new("git").current_dir(root).args(["status", "--porcelain"]))?,
        "rustc": capture(Command::new("rustc").arg("-Vv"))?,
        "binary": binary,
        "scenario": scenario.name(),
        "cycles": cycles,
        "events": events,
        "profile_event": profile_event,
        "profile_frequency": frequency,
        "stat_events": stat_events,
        "rustflags": env::var_os("RUSTFLAGS").map(|value| value.to_string_lossy().into_owned()),
        "encoded_rustflags": env::var_os("CARGO_ENCODED_RUSTFLAGS").map(|value| value.to_string_lossy().into_owned()),
        "frame_pointers": true,
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "available_parallelism": std::thread::available_parallelism()?.get(),
        "cpu_model": fs::read_to_string("/proc/cpuinfo").ok().and_then(|info| {
            info.lines().find_map(|line| line.strip_prefix("model name").map(str::trim).and_then(|line| line.strip_prefix(':')).map(str::trim).map(str::to_owned))
        }),
    });
    fs::write(
        directory.join("metadata.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    if matches!(mode, Mode::Run) {
        let mut command = Command::new(&binary);
        workload(
            &mut command,
            scenario,
            &directory.join("run.json"),
            cycles,
            events,
        );
        checked(&mut command)?;
    }
    if matches!(mode, Mode::Record | Mode::All) {
        let data = directory.join("perf.data");
        let mut command = Command::new("perf");
        command
            .args(["record", "--call-graph", "fp", "--freq"])
            .arg(frequency.to_string())
            .arg("--event")
            .arg(&profile_event)
            .arg("--output")
            .arg(&data)
            .arg("--")
            .arg(&binary);
        workload(
            &mut command,
            scenario,
            &directory.join("record.json"),
            cycles,
            events,
        );
        checked(&mut command)?;
        export_recording(&directory, &data)?;
    }
    if matches!(mode, Mode::Stat | Mode::All) {
        let mut command = Command::new("perf");
        command
            .args(["stat", "--event"])
            .arg(&stat_events)
            .arg("--output")
            .arg(directory.join("perf-stat.txt"))
            .arg("--")
            .arg(&binary);
        workload(
            &mut command,
            scenario,
            &directory.join("stat.json"),
            cycles,
            events,
        );
        checked(&mut command)?;
    }
    println!("Results: {}", directory.display());
    Ok(())
}

fn export_recording(directory: &Path, data: &Path) -> Result {
    checked(
        Command::new("perf")
            .args([
                "report",
                "--stdio",
                "--no-children",
                "--call-graph",
                "none",
                "--sort",
                "dso,symbol",
                "--input",
            ])
            .arg(data)
            .stdout(File::create(directory.join("perf-report.txt"))?),
    )?;
    checked(
        Command::new("perf")
            .args(["script", "--input"])
            .arg(data)
            .stdout(File::create(directory.join("perf-script.txt"))?),
    )?;
    Ok(())
}

fn compare(baseline: &Path, current: &Path) -> Result {
    let baseline: Value = serde_json::from_slice(&fs::read(baseline)?)?;
    let current: Value = serde_json::from_slice(&fs::read(current)?)?;
    for key in [
        "schema",
        "scenario",
        "events",
        "cycles",
        "frames",
        "viewport",
        "step_dt",
        "warmup_frames",
        "viewports",
        "frames_per_resize",
        "anchor_date",
        "route",
    ] {
        let required = !matches!(
            key,
            "viewports" | "frames_per_resize" | "anchor_date" | "route"
        );
        if (required && baseline.get(key).is_none()) || baseline.get(key) != current.get(key) {
            return Err(format!("Incompatible measurements: {key}").into());
        }
    }
    if baseline.get("storage") != current.get("storage") {
        return Err("Incompatible measurements: storage".into());
    }
    compare_metrics("", &baseline, &current)?;
    match (baseline.get("stages"), current.get("stages")) {
        (Some(before), Some(after)) if before.is_object() && after.is_object() => {
            if before
                .as_object()
                .unwrap()
                .keys()
                .ne(after.as_object().unwrap().keys())
            {
                return Err("Incompatible measurement stages".into());
            }
            for (stage, metrics) in before.as_object().unwrap() {
                compare_metrics(stage, metrics, &after[stage])?;
            }
        }
        (None, None) => {}
        _ => return Err("Incompatible measurement stages".into()),
    }
    Ok(())
}

fn compare_metrics(stage: &str, baseline: &Value, current: &Value) -> Result {
    if !stage.is_empty() {
        println!("\nStage: {stage}");
    }
    println!("Metric       Baseline ms   Current ms   Change");
    for key in [
        "mean_ms", "p50_ms", "p95_ms", "p99_ms", "max_ms", "total_ms",
    ] {
        let before = baseline[key]
            .as_f64()
            .filter(|value| *value > 0.0)
            .ok_or("Invalid baseline metric")?;
        let after = current[key]
            .as_f64()
            .filter(|value| *value >= 0.0)
            .ok_or("Invalid current metric")?;
        println!(
            "{key:12} {before:11.3} {after:12.3} {:+8.2}%",
            (after / before - 1.0) * 100.0
        );
    }
    Ok(())
}
