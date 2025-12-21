use std::collections::{HashMap, HashSet};
use std::{env, fs, process};
use std::io::{self, Write};
use std::time::Duration;
use std::thread;


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
	    // TODO: kinda redundant. A refactor of parse_proc_stat with a proper ProcStat struct would help.
		/*
    	if parts.len() < 5 {
    		continue;
    	}
   	    let ppid: i32 = match parts[4].parse::<i32>(){
   	    	Ok(ppid) => {ppid},
   	    	Err(_) => continue,
   	    };
   	    */
   	    map.insert(proc_stat.pid, proc_stat.ppid);
    }
    map
}


fn parse_statm(contents: String) -> Option<u64> {
    let parts: Vec<&str> = contents.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let rss_pages: u64 = match parts[1].parse::<u64>() {
        Ok(n) => n,
        Err(_) => return None,
    };
    // TODO: get the page size dynamically
    let page_size_kb = 4; // 4096 bytes = 4 KB
    Some(rss_pages * page_size_kb)
}


fn read_rss_kb(pid: &i32) -> Option<u64>{
    // see https://man7.org/linux/man-pages/man5/proc_pid_statm.5.html
    let path = format!("/proc/{}/statm", pid);
    /*
    TODO
    Trick 2: Use std::fs::read instead of read_to_string
    read_to_string incurs UTF-8 validation — wasteful since /proc is ASCII.
    */
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };
    parse_statm(contents)
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


enum Output {
	// to handle either stdout or a file
    File(fs::File),
    Stdout(io::Stdout),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(f) => f.write(buf),
            Self::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(f) => f.flush(),
            Self::Stdout(s) => s.flush(),
        }
    }
}


fn write_output<W: Write>(mut out: W, text: String){
    match out.write_all(text.as_bytes()){
		Ok(_) => (),
		Err(e) => {eprintln!("Could not write output because {}", e);}
    };
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "--help"{
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
    let print_flag: bool = !args.contains(&"--final".to_string());
    let mut hz: u64 = 1;
    if let Some(hz_index) = args.iter().position(|arg| arg == "--hertz") && args.len() > hz_index{
    	hz = args[hz_index + 1].parse().expect("Invalid strickly positive integer value for hertz option");
    }
    if hz == 0{
    	eprintln!("Invalid strickly positive integer value for hertz option");
    	process::exit(1);
    }
    let sleep_duration: u64 = 1000 / hz;

    let output_index = args.iter().position(|arg| arg == "--output-file");
    let mut output = if output_index.is_some_and(|index| args.len() > index) {
		Output::File(fs::File::create(args[output_index.unwrap() + 1].clone()).expect("Could not open output file"))
    } else{
		Output::Stdout(io::stdout())
    };

    let target_pid: i32 = args[args.len() -1].parse().expect("Invalid integer value for PID");

    let process_name = match get_process_name(target_pid) {
	    Ok(name) => name,
	    Err(msg) => {
	        eprintln!("memimpact error: {}", msg);
	        process::exit(1);
	    }
	};
	if print_flag{
	    write_output(&mut output, format!("Tracking memory usage of PID {} {}\n", target_pid, process_name));
	}

    let mut max: u64 = 0;
    let mut current: u64;

    loop {
        let mapping = get_map_pid_to_ppid();
        if !mapping.contains_key(&target_pid){
        	break;
        }
        let target_descendants = find_descendants(&mapping, target_pid);
        current = target_descendants.iter().map(|pid| read_rss_kb(pid).unwrap_or(0)).sum();
        
        max = max.max(current);
        let display_current = format_memory(current);
        let display_max = format_memory(max);
        if print_flag{
	        write_output(&mut output, format!("PID {} {}: current {}, max {}\n", target_pid, process_name, display_current, display_max ));
	    }
        thread::sleep(Duration::from_millis(sleep_duration));
    }
    let display_max = format_memory(max);
    write_output(&mut output, format!("PID {} {}: max {}\n", target_pid, process_name, display_max ));
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
        assert_eq!(parse_statm(input.to_string()), Some(200));
    }

    #[test]
    fn test_parse_statm_invalid() {
        assert_eq!(parse_statm("invalid".to_string()), None);
    }

    #[test]
    fn test_write_output_to_buffer() {
        let mut buffer: Vec<u8> = Vec::new();
        write_output(&mut buffer, "hello".to_string());
        assert_eq!(buffer, b"hello");
    }
}
