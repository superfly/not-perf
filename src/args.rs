use std::ffi::OsString;

use perf_event_open::EventSource;

use crate::cmd_collate::CollateFormat;

fn parse_event_source(source: &str) -> EventSource {
    match source {
        "hw_cpu_cycles" => EventSource::HwCpuCycles,
        "hw_ref_cpu_cycles" => EventSource::HwRefCpuCycles,
        "sw_cpu_clock" => EventSource::SwCpuClock,
        "sw_page_faults" => EventSource::SwPageFaults,
        "sw_dummy" => EventSource::SwDummy,
        _ => unreachable!(),
    }
}

fn parse_collate_format(format: &str) -> CollateFormat {
    match format {
        "collapsed" => CollateFormat::Collapsed,
        "perf-like" => CollateFormat::PerfLike,
        _ => unreachable!(),
    }
}

fn try_parse_period(period: &str) -> Result<u64, <u64 as std::str::FromStr>::Err> {
    let period = if period.ends_with("ms") {
        period[0..period.len() - 2].parse::<u64>()? * 1000_000
    } else if period.ends_with("us") {
        period[0..period.len() - 2].parse::<u64>()? * 1000
    } else if period.ends_with("ns") {
        period[0..period.len() - 2].parse::<u64>()?
    } else if period.ends_with("s") {
        period[0..period.len() - 1].parse::<u64>()? * 1000_000_000
    } else {
        period.parse::<u64>()? * 1000_000_000
    };

    Ok(period)
}

fn parse_period(period: &str) -> u64 {
    match try_parse_period(period) {
        Ok(period) => period,
        Err(_) => {
            eprintln!("error: invalid '--period' specified");
            std::process::exit(1);
        }
    }
}

pub enum TargetProcess {
    ByPid(u32),
    ByName(String),
    ByNameWaiting(String, u64),
}

#[derive(Clone, Debug)]
pub struct ProcessFilter {
    /// Profiles a process with a given PID (conflicts with --process)
    pub pid: Option<u32>,
    /// Profiles a process with a given name (conflicts with --pid)
    pub process: Option<String>,
    /// Will wait for the profiled process to appear
    pub wait: bool,
    /// Specifies the number of seconds which the profiler should wait
    /// for the process to appear; makes sense only when used with the `--wait` option
    pub wait_timeout: u32,
}

impl From<ProcessFilter> for TargetProcess {
    fn from(args: ProcessFilter) -> Self {
        if let Some(process) = args.process {
            if args.wait {
                TargetProcess::ByNameWaiting(process, args.wait_timeout as u64)
            } else {
                TargetProcess::ByName(process)
            }
        } else if let Some(pid) = args.pid {
            TargetProcess::ByPid(pid)
        } else {
            unreachable!();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Granularity {
    Address,
    Function,
    Line,
}

impl Default for Granularity {
    fn default() -> Self {
        Granularity::Line
    }
}

fn parse_granularity(value: &str) -> Granularity {
    match value {
        "address" => Granularity::Address,
        "function" => Granularity::Function,
        "line" => Granularity::Line,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct GenericProfilerArgs {
    /// The file to which the profiling data will be written
    pub output: Option<OsString>,

    /// The number of samples to gather; unlimited by default
    pub sample_count: Option<u64>,

    /// Determines for how many seconds the measurements will be gathered
    pub time_limit: Option<u64>,

    /// Prevents anything in the profiler's address space from being swapped out; might increase memory usage significantly
    pub lock_memory: bool,

    /// Disable online backtracing
    pub offline: bool,

    pub panic_on_partial_backtrace: bool,

    pub process_filter: ProcessFilter,
}

#[derive(Debug)]
pub struct RecordArgs {
    /// The frequency with which the measurements will be gathered
    pub frequency: u32,

    /// The source of perf events
    pub event_source: Option<EventSource>,

    /// Size of the gathered stack payloads (in bytes)
    pub stack_size: u32,

    /// Gather data but do not do anything with it; useful only for testing
    pub discard_all: bool,

    pub profiler_args: GenericProfilerArgs,
}

#[derive(Debug)]
pub struct SharedCollationArgs {
    /// A file or directory with extra debugging symbols; can be specified multiple times
    pub debug_symbols: Vec<OsString>,

    /// A path to a jitdump file
    pub jitdump: Option<OsString>,

    pub force_stack_size: Option<u32>,

    pub omit: Vec<String>,

    pub only_sample: Option<u64>,

    /// Completely ignores kernel callstacks
    pub without_kernel_callstacks: bool,

    /// Only process the samples generated *after* this many seconds after launch.
    pub from: Option<String>,

    /// Only process the samples generated *before* this many seconds after launch.
    pub to: Option<String>,

    /// The input file to use; record it with the `record` subcommand
    pub input: OsString,
}

#[derive(Debug)]
pub struct ArgMergeThreads {
    /// Merge callstacks from all threads
    pub merge_threads: bool,
}

#[derive(Debug)]
pub struct ArgGranularity {
    /// Specifies at what granularity the call frames will be merged
    pub granularity: Granularity,
}

#[cfg(feature = "inferno")]
#[derive(Debug)]
pub struct FlamegraphArgs {
    pub collation_args: SharedCollationArgs,

    pub arg_merge_threads: ArgMergeThreads,

    pub arg_granularity: ArgGranularity,

    /// The file to which the flamegraph will be written to (instead of the stdout)
    pub output: Option<OsString>,
}

#[derive(Debug)]
pub struct CsvArgs {
    pub collation_args: SharedCollationArgs,

    /// The sampling interval, in seconds
    pub sampling_interval: Option<f64>,

    /// The file to which the CSV will be written to (instead of the stdout)
    pub output: Option<OsString>,
}

#[derive(Debug)]
pub struct TraceEventsArgs {
    pub collation_args: SharedCollationArgs,

    pub arg_granularity: ArgGranularity,

    pub absolute_time: bool,

    /// The sampling period; samples within one sampling period will be merged together
    pub period: Option<u64>,

    /// The file to which the trace events will be written to
    pub output: OsString,
}

#[derive(Debug)]
pub struct CollateArgs {
    pub collation_args: SharedCollationArgs,

    pub arg_merge_threads: ArgMergeThreads,

    pub arg_granularity: ArgGranularity,

    /// Selects the output format
    pub format: CollateFormat,
}

#[derive(Debug)]
pub struct MetadataArgs {
    /// The input file to use; record it with the `record` subcommand
    pub input: OsString,
}

#[derive(Debug)]
pub enum Opt {
    /// Records profiling information with perf_event_open
    Record(RecordArgs),

    /// Emits an SVG flamegraph
    #[cfg(feature = "inferno")]
    Flamegraph(FlamegraphArgs),

    /// Emits a CSV file
    Csv(CsvArgs),

    /// Emits trace events for use with Chromium's Trace Viewer
    TraceEvents(TraceEventsArgs),

    /// Emits collated stack traces for use with Brendan Gregg's flamegraph script
    Collate(CollateArgs),

    /// Outputs rudimentary JSON-formatted metadata
    Metadata(MetadataArgs),
}
