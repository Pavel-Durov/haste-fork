use crate::BenchKey;
use crate::{ResultFile, config::*};
use std::hint::black_box;
use std::io::{self, Write};
use std::path::Path;
use std::process::{self, Command, Stdio};
use std::time::Duration;
use terminal_size::terminal_size;

/// Number of iterations to pass to harness.lua (always 1, since haste handles
/// multiple runs via proc_execs).
const HARNESS_NUM_ITERATIONS: &str = "1";

/// Build the command arguments for lua harness benchmark execution.
///
/// Returns a vector of arguments that will be passed to the lua harness.
/// The format is: [harness, bench_name, num_iterations, inner_iterations, ...extra_args]
/// where num_iterations is always "1" (haste handles multiple runs via proc_execs)
/// and inner_iterations is the value of inproc_iters. (https://github.com/ykjit/yk-benchmarks/blob/main/suites/awfy/Lua/harness.lua#L32)
fn build_benchmark_args(
    harness: &str,
    bench_name: &str,
    inproc_iters: usize,
    extra_args: &[String],
) -> Vec<String> {
    let mut args = vec![
        harness.to_string(),
        bench_name.to_string(),
        HARNESS_NUM_ITERATIONS.to_string(),
        inproc_iters.to_string(),
    ];
    args.extend(extra_args.iter().cloned());
    args
}

fn total_pexecs(config: &Config) -> usize {
    let mut total_pexecs = 0;
    for suite in &config.suites {
        total_pexecs += suite.1.benchmarks.len();
    }
    total_pexecs *= config.executors.len();
    total_pexecs *= config.proc_execs;
    total_pexecs
}

fn get_progress_percentage(config: &Config, completed_pexecs: usize) -> f64 {
    let completed_pexecs = f64::from(u32::try_from(completed_pexecs).unwrap());
    let total_pexecs = f64::from(u32::try_from(total_pexecs(config)).unwrap());
    completed_pexecs / total_pexecs * 100.
}

fn get_eta(config: &Config, results: &ResultFile, completed_pexecs: usize) -> String {
    if completed_pexecs == 0 {
        return "...".to_owned();
    }
    let msecs = (results.data.values().flatten().sum::<f64>()
        / f64::from(u32::try_from(completed_pexecs).unwrap()))
        * f64::from(u32::try_from(total_pexecs(config) - completed_pexecs).unwrap());
    let dur = Duration::from_millis(msecs as u64);
    let secs = dur.as_secs();
    if secs >= 24 * 60 * 60 {
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        format!("{}d:{}h", days, hours)
    } else if secs < 60 * 60 {
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        format!("{}m:{:02}s", minutes, seconds)
    } else {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        format!("{}H:{:02}m", hours, minutes)
    }
}

/// Run all benchmarks from the configuration.
pub(crate) fn run(config: &Config) -> ResultFile {
    let mut results = ResultFile::default();
    let mut completed_pexecs = 0;
    for (executor_name, executor) in &config.executors {
        for suite in &config.suites {
            run_suite(
                &mut results,
                config,
                &mut completed_pexecs,
                executor_name,
                executor,
                suite.1,
            );
        }
    }
    results
}

fn hide_cursor() {
    let mut out = io::stdout();
    write!(out, "\x1B[?25l").ok(); // hide
    out.flush().ok();
}

fn show_cursor() {
    let mut out = io::stdout();
    write!(out, "\x1B[?25h").ok(); // hide
    out.flush().ok();
}

fn update_term_line(lhs: &str, rhs: &str) {
    let width = terminal_size()
        .map(|(width, _height)| usize::from(width.0))
        .unwrap_or(80);
    let lhs_c = lhs.chars().count();
    let rhs_c = rhs.chars().count();
    if lhs_c + 1 + rhs_c <= width {
        let spc = " ".repeat(width - lhs.chars().count() - rhs.chars().count());
        print!("\r{lhs}{spc}{rhs}");
    } else if width < rhs_c + 1 {
        // If the user's got a ludicrously narrow terminal, nothing we do will work very well, so
        // don't try hard.
        print!("\r{lhs} {rhs}");
    } else {
        // If the terminal is a bit too narrow, chop the LHS down and retain all of the RHS.
        let lhs_cutdown = lhs.chars().take(width - rhs_c - 1).collect::<String>();
        print!("\r{lhs_cutdown} {rhs}");
    }
}

/// Run a suite with the specified executor.
fn run_suite(
    results: &mut ResultFile,
    config: &Config,
    completed_pexecs: &mut usize,
    executor_name: &str,
    executor: &Path,
    suite: &Suite,
) {
    hide_cursor();
    ctrlc::set_handler(show_cursor).ok();
    for (bench_name, bench) in &suite.benchmarks {
        let key = BenchKey {
            benchmark: bench_name.into(),
            executor: executor_name.into(),
            extra_args: bench.extra_args.clone(),
        };
        let progress = get_progress_percentage(config, *completed_pexecs);
        let eta = get_eta(config, results, *completed_pexecs);
        update_term_line(
            &format!(">>> haste: {key} ..."),
            &format!("{:3.0}% (ETA {eta})", progress.round() as i64),
        );

        for i in 0..(config.proc_execs) {
            io::stdout().flush().ok();
            run_benchmark(
                results,
                config,
                executor_name,
                executor,
                suite,
                bench_name,
                bench,
            );
            *completed_pexecs += 1;
            let progress = get_progress_percentage(config, *completed_pexecs);
            let eta = get_eta(config, results, *completed_pexecs);
            let so_far = results
                .data
                .get(&key.to_string())
                .unwrap()
                .iter()
                .map(|x| format!("{x}ms"))
                .collect::<Vec<_>>()
                .join(" ");
            let lhs = if i + 1 < config.proc_execs {
                format!(">>> haste: {key} {so_far} ...")
            } else {
                format!(">>> haste: {key} {so_far}")
            };
            let rhs = if i + 1 < config.proc_execs {
                format!("{:3.0}% (ETA {eta})", progress.round() as i64)
            } else {
                "".to_owned()
            };
            update_term_line(&lhs, &rhs);
        }
        println!();
    }
    show_cursor();
}

/// Run an individual benchmark.
fn run_benchmark(
    results: &mut ResultFile,
    config: &Config,
    executor_name: &str,
    executor: &Path,
    suite: &Suite,
    bench_name: &str,
    bench: &Benchmark,
) {
    let harness = suite.harness.to_str().unwrap();
    let args = build_benchmark_args(harness, bench_name, config.inproc_iters, &bench.extra_args);

    let mut cmd = Command::new(executor);
    cmd.current_dir(&suite.dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &suite.env {
        cmd.env(k, v);
    }
    cmd.args(args.iter().map(String::as_str));

    let t = std::time::Instant::now();
    // We are careful to use `output()` and not `spawn()` here so as to avoid deadlocks for
    // benchmarks that make a lot of output.
    let Ok(output) = black_box(cmd.output()) else {
        eprintln!("error: failed to spawn benchmark!");
        eprintln!("args: {cmd:?}");
        show_cursor();
        process::exit(1)
    };

    let elapsed = f64::from(u32::try_from(t.elapsed().as_millis()).unwrap());

    if !output.status.success() {
        println!();
        eprintln!("error: benchmark command exited non-zero!");
        eprintln!("args: {cmd:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("--- Begin stdout ---");
        eprint!("{stdout}");
        eprintln!("--- End stdout ---");
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("--- Begin stderr ---");
        eprint!("{stderr}");
        eprintln!("--- End stderr ---");
        show_cursor();
        process::exit(1)
    }

    let bench_key = BenchKey {
        benchmark: bench_name.to_owned(),
        executor: executor_name.to_owned(),
        extra_args: bench.extra_args.to_owned(),
    };
    results
        .data
        .entry(bench_key.to_string())
        .or_default()
        .push(elapsed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_benchmark_args_basic() {
        let args = build_benchmark_args("harness.lua", "deltablue", 10, &[]);
        assert_eq!(args, vec!["harness.lua", "deltablue", "1", "10"]);
    }

    #[test]
    fn test_build_benchmark_args_with_extra_args() {
        let args = build_benchmark_args("harness.lua", "deltablue", 10, &["12000".to_string()]);
        assert_eq!(args, vec!["harness.lua", "deltablue", "1", "10", "12000"]);
    }

    #[test]
    fn test_build_benchmark_args_num_iterations_always_one() {
        // Verify that num_iterations is always "1" regardless of inproc_iters
        let args1 = build_benchmark_args("harness.lua", "bench", 1, &[]);
        let args2 = build_benchmark_args("harness.lua", "bench", 100, &[]);
        assert_eq!(args1[2], HARNESS_NUM_ITERATIONS.to_string());
        assert_eq!(args2[2], HARNESS_NUM_ITERATIONS.to_string());
    }
    #[test]
    fn test_build_benchmark_args_argument_order() {
        let args = build_benchmark_args("harness.lua", "deltablue", 10, &["12000".to_string()]);
        assert_eq!(args[0], "harness.lua"); // harness path
        assert_eq!(args[1], "deltablue"); // benchmark name (arg[1] in harness.lua)
        assert_eq!(args[2], "1"); // num_iterations (arg[2] in harness.lua)
        assert_eq!(args[3], "10"); // inner_iterations (arg[3] in harness.lua)
        assert_eq!(args[4], "12000"); // extra_args[0] (arg[4] in harness.lua)
    }
}
