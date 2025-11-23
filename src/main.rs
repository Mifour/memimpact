use std::collections::{HashMap, HashSet};
use std::{env, fs, process};
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


fn parse_proc_stat(content: &str) -> Vec<&str>{
	// because the 2nd colum is the process name and can contain whitespaces
	// see https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html
	let mut res = Vec::<&str>::new();
	let open = match content.find('(') {
        Some(i) => i,
        None => return res,
    };
    res.push(&content[0..open - 1]);
    let close = match content[open + 1..].find(')') {
        Some(i) => open + 1 + i,
        None => return res,
    };
    res.push(&content[open..close + 1]);
    let after_comm = close + 3;

    let rest: Vec<&str> = content[after_comm..].split_whitespace().collect();
	res.extend(rest);
    res
}

fn get_process_name(pid: &i32) -> String{
	let path = format!("/proc/{}/stat", pid);
   	let contents = match fs::read_to_string(path){
   		Ok(string_content) => {string_content},
   		Err(_) => {panic!("could not read process name for pid {}", pid)}	
   	};
   	let parts: Vec<&str> = parse_proc_stat(&contents);
   	if parts.len() < 2{
   		panic!("could not get the process name for stat: {:?}", parts);
   	}
   	parts[1].to_string()
}

fn get_map_pid_to_ppid() -> HashMap<i32, i32> {
    // list directories insde /proc and foreach read its stat
    // returns a map of i32 -> i32, each representing a pid to its ppid 
    let mut map = HashMap::<i32, i32>::new();
    for pid in list_processes(){
    	let path = format!("/proc/{}/stat", pid);
    	let contents = match fs::read_to_string(path){
    		Ok(string_content) => {string_content},
    		Err(_) => {continue} // probably the process exited	
    	};
    	// TODO: Parse /proc/<pid>/stat manually to avoid allocations
    	let parts: Vec<&str> = parse_proc_stat(&contents);
    	if parts.len() >= 4 {
    	    let ppid: i32 = match parts[3].parse::<i32>(){
    	    	Ok(ppid_int) => {ppid_int},
    	    	Err(error) => {
    	    	    panic!("cannot parse {:?} from {:?} got error {:?}", parts[3], &parts[..10], error)
    	    	}
    	    };
    	    map.insert(pid, ppid);        
    	}
    }
    map
}

fn read_rss_kb(pid: &i32) -> Option<u64>{
    // see https://man7.org/linux/man-pages/man5/proc_pid_statm.5.html
    let path = format!("/proc/{}/statm", pid);
    /*
    TODO
    Trick 2: Use std::fs::read instead of read_to_string
    read_to_string incurs UTF-8 validation — wasteful since /proc is ASCII.
    */
    let contents = fs::read_to_string(path).ok()?;
    let parts: Vec<&str> = contents.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    // this the physical memory allocated to a process (this includes its threads)
    let rss_pages: u64 = parts[1].parse::<u64>().ok()?;
    // TODO: get the page size dynamically
    let page_size_kb = 4; // 4096 bytes = 4 KB
	Some(rss_pages * page_size_kb)
}	


fn find_descendants(
    parent_of: &HashMap<i32, i32>,
    target_pid: i32,
) -> HashSet<i32> {
	// Given a mapping of pid -> ppid and a target pid,
	// return all descendants of the target (including the target itself)
    let mut descendants = HashSet::new();
    descendants.insert(target_pid);
	let mut found_new = false;
    loop {
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
	let mut current = value;
	let mut power: u32 = 3; // start in KB
	while current > 1024{
		current >>= 10; // divide by 1024
		power += 3;
		if power > 21{
			panic!("format_memory is stuck in while loop");
		}
	}
	let unit_str = match power {
		3 => {"KB"},
		6 => {"MB"},
		9 => {"GB"},
		12 => {"TB"},
		15 => {"PB"},
		18 => {"EB"}, 
		21 => {"ZB"},
		_ => {panic!("unit name for power {} is not supported for conversion", power)}  // impossible with a u64, value would be > than u64 max
	};
	format!("{}{}", current, unit_str).to_string()
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

    let target_pid: i32 = args[args.len() -1].parse().expect("Invalid integer value for PID");

    let process_name: String = get_process_name(&target_pid);

	if print_flag{
	    println!("Tracking memory usage of PID {} {}", target_pid, process_name);
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
	        println!("PID {} {}: current {}, max {}", target_pid, process_name, display_current, display_max );
	    }
        thread::sleep(Duration::from_millis(sleep_duration));
    }
    let display_max = format_memory(max);
    println!("PID {} {}: max {}", target_pid, process_name, display_max );
}
