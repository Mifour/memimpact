use std::collections::{HashMap, HashSet};
use std::{env, fs, process};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;
use std::thread;


// TODO: get the page size dynamically
const PAGE_SIZE_KB: u64 = 4; // 4096 bytes = 4 KB

fn list_processes() -> Vec<i32> {
    let mut pids = Vec::new();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {              // ignore invalid directory entries
            if let Ok(metadata) = entry.metadata() && metadata.is_dir() {  // ignore metadata errors
               if let Some(name) = entry.file_name().to_str() {
                    if let Ok(pid) = name.parse::<i32>() {
                        pids.push(pid);
                    }
                }
            }
        }
    }
    pids
}

#[derive(Debug, PartialEq)]
enum ProcessState{
	R,      //Running
    S,      //Sleeping in an interruptible wait
    D,      //Waiting in uninterruptible disk sleep
    Z,      //Zombie
    T,      //Stopped (on a signal) or (before Linux2.6.33) trace stopped or Tracing stop (Linux 2.6.33 onward)
    W,      //Paging (only before Linux 2.6.0) or Waking (Linux 2.6.33 to 3.13 only)
    X,      //Dead (from Linux 2.6.0 onward)
    K,      //Wakekill (Linux 2.6.33 to 3.13 only)
    P,      //Parked (Linux 3.9 to 3.13 only)
    I,      //Idle (Linux 4.14 onward)
}


impl TryFrom<&str> for ProcessState {
    type Error = ProcStatError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.chars().next().ok_or('_') {
            Ok('R') => Ok(ProcessState::R),
            Ok('S') => Ok(ProcessState::S),
            Ok('D') => Ok(ProcessState::D),
            Ok('Z') => Ok(ProcessState::Z),
            Ok('T') => Ok(ProcessState::T),
            Ok('W') => Ok(ProcessState::W),
            Ok('X') => Ok(ProcessState::X),
            Ok('K') => Ok(ProcessState::K),
            Ok('P') => Ok(ProcessState::P),
            Ok('I') => Ok(ProcessState::I),
            _ => Err(ProcStatError::UnsupportedKernelLayout),
        }
    }
}



#[derive(Debug, PartialEq)]
#[allow(dead_code)]
struct ProcStat<'a>{
    pid: i32,
    comm: &'a str,
    state: ProcessState,
    ppid: i32,
}


#[derive(Debug)]
enum ProcStatError {
    InvalidFormat,
    UnsupportedKernelLayout,
}


fn parse_proc_stat(content: &str) -> Result<ProcStat<'_>, ProcStatError> {
	// because the 2nd colum is the process name and can contain whitespaces
	// see https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html
    let mut res = Vec::new();

    let open = content.find('(').ok_or(ProcStatError::InvalidFormat)?;
    let close = content[open + 1..]
        .find(')')
        .map(|i| open + 1 + i)
        .ok_or(ProcStatError::InvalidFormat)?;

    // pid
    if open < 2 {
        return Err(ProcStatError::InvalidFormat);
    }
    res.push(&content[..open - 1]);
	let pid: i32 = match content[..open - 1].parse(){
		Ok(i) => i,
		Err(_) => return Err(ProcStatError::InvalidFormat)
	};

	// comm
    let comm = &content[open..=close];

	// state
    let after_comm = close + 2;
    let state = match ProcessState::try_from(&content[after_comm..after_comm + 1]){
    	Ok(s) => s,
    	Err(_) => return Err(ProcStatError::UnsupportedKernelLayout)
    };

    // ppid
    let next_space = content[after_comm + 2..].find(' ').ok_or(ProcStatError::InvalidFormat)?;
	let ppid: i32 = match content[after_comm + 2..after_comm + 2 + next_space].parse(){
		Ok(i) => i,
		Err(_) => return Err(ProcStatError::InvalidFormat)
	};
    
    Ok(ProcStat{pid, comm, state, ppid})
}

fn get_process_name(pid: i32) -> Result<String, String> {
    let path = format!("/proc/{}/stat", pid);
    let contents = fs::read_to_string(&path)
   	        .map_err(|_| format!("Could not read {}", path))?;
    let proc_stat = parse_proc_stat(&contents).map_err(|e| {
        format!(
            "Unsupported /proc/{}/stat format ({:?}). \
             Your system is currently not supported. \
             Please open an issue with your kernel version.",
            pid, e
        )
    })?;

    Ok(proc_stat.comm.to_string())
}


fn get_map_pid_to_ppid() -> HashMap<i32, i32> {
    // list directories insde /proc and foreach read its stat
    // returns a map of i32 -> i32, each representing a pid to its ppid 
    let mut map = HashMap::<i32, i32>::new();
    for pid in list_processes(){
    	let path = format!("/proc/{}/stat", pid);
    	let contents = match fs::read_to_string(path){
    		Ok(c) => {c},
    		Err(_) => {continue} // probably the process exited	
    	};
    	let proc_stat = match parse_proc_stat(&contents) {
	        Ok(p) => p,
	        Err(_) => continue, // unsupported or malformed stat for this PID
	    };
   	    map.insert(proc_stat.pid, proc_stat.ppid);
    }
    map
}


#[derive(Debug)]
enum ProcStatmError {
    InvalidFormat,
}


fn parse_statm(content: String) -> Result<u64, ProcStatmError> {
	let first_space = match content.find(' ').ok_or(ProcStatmError::InvalidFormat){
		Ok(i) => i,
		Err(_) => return Err(ProcStatmError::InvalidFormat)
	};
	let next_space = match content[first_space + 1..].find(' ').ok_or(ProcStatmError::InvalidFormat){
		Ok(i) => i,
		Err(_) => return Err(ProcStatmError::InvalidFormat)
	};
    let rss_pages: u64 = match content[first_space + 1..first_space + 1 + next_space].parse::<u64>() {
        Ok(n) => n,
        Err(_) => return Err(ProcStatmError::InvalidFormat),
    };

    Ok(rss_pages * PAGE_SIZE_KB)
}


fn read_rss_kb(pid: &i32) -> u64{
    // see https://man7.org/linux/man-pages/man5/proc_pid_statm.5.html
    let path = format!("/proc/{}/statm", pid);
    /*
    TODO
    Trick 2: Use std::fs::read instead of read_to_string
    read_to_string incurs UTF-8 validation — wasteful since /proc is ASCII.
    */
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    parse_statm(contents).unwrap_or(0)
}	


fn find_descendants(
    parent_of: &HashMap<i32, i32>,
    target_pid: i32,
) -> HashSet<i32> {
	// Given a mapping of pid -> ppid and a target pid,
	// return all descendants of the target (including the target itself)
    let mut descendants = HashSet::new();
    descendants.insert(target_pid);
	let mut found_new: bool;
    loop {
    	found_new = false;
        for (&pid, &ppid) in parent_of.iter() {
        	// if the parent process is among descendants and we don't already know the current pid
            if descendants.contains(&ppid) && !descendants.contains(&pid) {
                descendants.insert(pid);
                found_new = true;
            }
        }
        if !found_new {
            break;
        }
    }
    descendants
}


fn format_memory(value: u64) -> String{
	// every possible u64 values are handled, it is impossible to be stuck in an infinite loop
	const UNITS: [&str; 7] = ["KB", "MB", "GB", "TB", "PB", "EB", "ZB"];
    let mut current = value;
    let mut unit_index = 0;
    while current >= 1024 && unit_index < UNITS.len() - 1 {
        current >>= 10;
        unit_index += 1;
    }
    format!("{}{}", current, UNITS[unit_index])
}


#[derive(Debug)]
enum OutputSpec {
    Stdout,
    File(PathBuf),
}

#[derive(Debug)]
enum Output {
    File(fs::File),
    Stdout(io::Stdout),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::File(f) => f.write(buf),
            Output::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::File(f) => f.flush(),
            Output::Stdout(s) => s.flush(),
        }
    }
}


fn write_output<W: Write>(mut out: W, text: String){
    match out.write_all(text.as_bytes()){
		Ok(_) => (),
		Err(e) => {eprintln!("Could not write output because {}", e);}
    };
}


fn setup_output(spec: OutputSpec) -> io::Result<Output> {
    match spec {
        OutputSpec::Stdout => Ok(Output::Stdout(io::stdout())),
        OutputSpec::File(path) => {
            let file = fs::File::create(path)?;
            Ok(Output::File(file))
        }
    }
}



#[derive(Debug)]
#[allow(dead_code)]
enum ParseArgError {
    MissingValue(&'static str),
    InvalidValue(&'static str),
}

#[derive(Debug)]
struct Args{
	help_flag: bool,
	final_flag: bool,
	hz: u64,
	output: OutputSpec,
	target_pid: i32,
}


fn parse_args(args: &[String]) -> Result<Args, ParseArgError> {
    let mut help_flag = false;
    let mut final_flag = false;
    let mut hz = 1;
    let mut output = OutputSpec::Stdout;
    let mut pid = None;

    let mut iter = args.iter().skip(1).peekable(); // skip program name

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" => help_flag = true,
            "--final" => final_flag = true,
            "--hertz" => {
                let value = iter.next().ok_or(ParseArgError::MissingValue("hertz"))?;
                hz = value.parse().map_err(|_| ParseArgError::InvalidValue("hertz"))?;
                if hz == 0 {
                    return Err(ParseArgError::InvalidValue("hertz"));
                }
            }
            "--output-file" => {
                let value = iter.next().ok_or(ParseArgError::MissingValue("output-file"))?;
                output = OutputSpec::File(PathBuf::from(value));
            }
            other => {
                // assume PID if numeric
                pid = Some(other.parse().map_err(|_| ParseArgError::InvalidValue("pid"))?);
            }
        }
    }

    let target_pid = pid.ok_or(ParseArgError::MissingValue("pid"))?;

    Ok(Args {
        help_flag,
        final_flag,
        hz,
        output,
        target_pid,
    })
}

fn main() {
	let raw_args: Vec<String> = env::args().collect();
    let args: Args = match parse_args(&raw_args) {
    	Ok(args_struct) => args_struct,
    	Err(e) => {
    		eprintln!("Memimpact failed to parsed arguments: {:?}", e);
    		process::exit(1);
    	}
    };
    if args.help_flag{
    	let version = env!("CARGO_PKG_VERSION");
    	println!(
			"Memimpact -- measure the memory impact of any PID and its children processes.\n\
			Version: {}\n\
			Usage: memimpact <options> <pid>\n\
			Options:\n\
			--hertz int, the desired number of iterations per second\n\
			--output-file str, the file path where to write the output (stdout if absent)\n\
			Flags:\n\
			--final, display only 1 line with the max value",
    		version
    	);
    	process::exit(0);
    }
    
	let sleep_duration: u64 = 1000 / args.hz;

    let process_name = match get_process_name(args.target_pid) {
	    Ok(name) => name,
	    Err(msg) => {
	        eprintln!("memimpact error: {}", msg);
	        process::exit(1);
	    }
	};

	let mut output = match setup_output(args.output) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Memimapct ailed to open output: {}", e);
            process::exit(1);
        }
    };
	
	if !args.final_flag{
	    write_output(
	    	&mut output,
	    	format!("Tracking memory usage of PID {} {}\n", args.target_pid, process_name)
	    );
	}

    let mut max: u64 = 0;
    let mut current: u64;

    loop {
        let mapping = get_map_pid_to_ppid();
        if !mapping.contains_key(&(args.target_pid)){
        	break;
        }
        let target_descendants = find_descendants(&mapping, args.target_pid);
        current = target_descendants.iter().map(read_rss_kb).sum();
        
        max = max.max(current);
        let display_current = format_memory(current);
        let display_max = format_memory(max);
        if !args.final_flag{
	        write_output(
	        	&mut output,
	        	format!(
	        		"PID {} {}: current {}, max {}\n",
	        		args.target_pid,
	        		process_name,
	        		display_current,
	        		display_max
	        	)
	        );
	    }
        thread::sleep(Duration::from_millis(sleep_duration));
    }
    let display_max = format_memory(max);
    write_output(
    	&mut output,
    	format!("PID {} {}: max {}\n", args.target_pid, process_name, display_max )
    );
}


/// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proc_stat_basic() {
        let input = "1234 (bash) R 1 2 3 4";
        let actual = parse_proc_stat(input).unwrap();

        let expected = ProcStat{pid: 1234, comm: "(bash)", state: ProcessState::R, ppid: 1};
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_proc_stat_with_spaces_in_name() {
        let input = "5678 (my fancy process) S 10 20 30";
        let actual = parse_proc_stat(input).unwrap();

        let expected = ProcStat{pid: 5678, comm: "(my fancy process)", state: ProcessState::S, ppid: 10};
        assert_eq!(actual, expected);
    }


    #[test]
    fn test_parse_proc_stat_invalid_missing_parens() {
        let input = "9999 bash R 1 2 3";
        let parts = parse_proc_stat(input);

        assert!(parts.is_err());
    }

    #[test]
    fn test_find_descendants_simple_tree() {
        let mut map = HashMap::new();
        map.insert(2, 1);
        map.insert(3, 1);
        map.insert(4, 2);
        map.insert(5, 4);

        let descendants = find_descendants(&map, 1);

        let expected: HashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
        assert_eq!(descendants, expected);
    }

    #[test]
    fn test_find_descendants_leaf() {
        let mut map = HashMap::new();
        map.insert(2, 1);
        map.insert(3, 1);

        let descendants = find_descendants(&map, 2);

        let expected: HashSet<i32> = [2].into_iter().collect();
        assert_eq!(descendants, expected);
    }

    #[test]
    fn test_format_memory_kb() {
        assert_eq!(format_memory(512), "512KB");
    }

    #[test]
    fn test_format_memory_mb() {
        assert_eq!(format_memory(2 * 1024), "2MB");
    }

    #[test]
    fn test_format_memory_gb() {
        assert_eq!(format_memory(2 * 1024 * 1024), "2GB");
    }

    #[test]
    fn test_format_memory_rounding_behavior() {
        assert_eq!(format_memory(1536), "1MB");
    }

    #[test]
    fn test_format_memory_max() {
        assert_eq!(format_memory(u64::MAX), "15ZB");
    }

    #[test]
    fn test_parse_statm_valid() {
        let input = "100 50 0 0 0 0 0";
        assert_eq!(parse_statm(input.to_string()).ok(), Some(200));
    }

    #[test]
    fn test_parse_statm_invalid() {
        assert!(parse_statm("invalid".to_string()).is_err());
    }

    #[test]
    fn test_write_output_to_buffer() {
        let mut buffer: Vec<u8> = Vec::new();
        write_output(&mut buffer, "hello".to_string());
        assert_eq!(buffer, b"hello");
    }

    fn args(input: &[&str]) -> Vec<String> { // to avoid to add .to_string in following argument tests
        input.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn minimal_valid_args() {
        let argv = args(&["memimpact", "1234"]);
        let parsed = parse_args(&argv).unwrap();

        assert_eq!(parsed.help_flag, false);
        assert_eq!(parsed.final_flag, false);
        assert_eq!(parsed.hz, 1);
        matches!(parsed.output, OutputSpec::Stdout);
        assert_eq!(parsed.target_pid, 1234);
    }

    #[test]
    fn full_valid_args() {
        let argv = args(&[
            "memimpact",
            "--hertz", "10",
            "--output-file", "out.txt",
            "--final",
            "4321",
        ]);

        let parsed = parse_args(&argv).unwrap();

        assert!(parsed.final_flag);
        assert!(!parsed.help_flag);
        assert_eq!(parsed.hz, 10);
        assert_eq!(parsed.target_pid, 4321);

        match parsed.output {
            OutputSpec::File(path) => assert_eq!(path, PathBuf::from("out.txt")),
            _ => panic!("expected file output"),
        }
    }

    #[test]
    fn help_flag_only() {
        let argv = args(&["memimpact", "--help", "999"]);

        let parsed = parse_args(&argv).unwrap();
        assert!(parsed.help_flag);
        assert_eq!(parsed.target_pid, 999);
    }

    #[test]
    fn hertz_value_missing_pid() {
        let argv = args(&["memimpact", "--hertz", "1234"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::MissingValue("pid") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn missing_hertz_value() {
        let argv = args(&["memimpact", "1234", "--hertz"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::MissingValue("hertz") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }


    #[test]
    fn invalid_hertz_value() {
        let argv = args(&["memimpact", "--hertz", "abc", "123"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::InvalidValue("hertz") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn zero_hertz_is_invalid() {
        let argv = args(&["memimpact", "--hertz", "0", "123"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::InvalidValue("hertz") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn missing_output_file_value() {
        let argv = args(&["memimpact", "1234", "--output-file"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::MissingValue("output-file") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn invalid_pid() {
        let argv = args(&["memimpact", "not_a_pid"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::InvalidValue("pid") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn missing_pid() {
        let argv = args(&["memimpact", "--final"]);

        let err = parse_args(&argv).unwrap_err();

        match err {
            ParseArgError::MissingValue("pid") => (),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn realistic_mixed_order() {
        let argv = args(&[
            "memimpact",
            "--final",
            "5678",
            "--hertz", "5",
        ]);

        let parsed = parse_args(&argv).unwrap();

        assert!(parsed.final_flag);
        assert_eq!(parsed.hz, 5);
        assert_eq!(parsed.target_pid, 5678);
    }

    #[test]
    fn realistic_order() {
        let argv = args(&[
            "memimpact",
            "--final",
            "--hertz", "5",
            "5678",
        ]);

        let parsed = parse_args(&argv).unwrap();

        assert!(parsed.final_flag);
        assert_eq!(parsed.hz, 5);
        assert_eq!(parsed.target_pid, 5678);
    }
}
