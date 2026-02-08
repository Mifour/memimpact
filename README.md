# MemImpact

**MemImpact** is a lightweight Linux CLI utility written in Rust that samples and reports the peak RSS memory usage of a process tree over time.  

It observes memory from the outside using the Linux `/proc` filesystem — similar in spirit to `top` or `ps` — and is designed for quick, low-friction memory measurements, not deep profiling.  

MemImpact answers:

> “Roughly how much RAM did this program (and its children) consume at peak?”

without requiring instrumentation, recompilation, or heavyweight tooling.  

## Demo

<p align="center">
  <img src="assets/demo.svg" />
</p>


### What MemImpact is
- A process memory monitor
- A peak RSS estimator
- A tool for quick benchmarking and sanity checks
- Useful in scripts, CI, experiments, and comparisons
- A Linux tool
  
### What MemImpact is NOT
- Not a memory profiler
- Not allocation tracing
- Not page-fault analysis
- Not precise down-to-the-microsecond peak capture
- Not desinged for MacOS or Windows
  
It samples memory at intervals, so extremely short spikes between samples may be missed.  

---

## Features

-  Measures peak memory usage (RSS) of any command  
-  Includes memory from **child processes**  
-  Works on any Linux (uses `/proc/<pid>/status`)  
-  Fast, zero external dependencies, written in pure Rust std  
-  Works as a direct command or with an optional shell wrapper  
-  Easy to install
-  Easy to use  
-  Templating for custom output formats

---

##  Installation

### **Option 1 - Install via Cargo**

```sh
cargo install --git https://github.com/mifour/memimpact
```
or
```sh
cargo install memimpact
```
### **Option 2 — Download a Prebuilt Binary**

Go to:

**GitHub Releases → Latest → Assets**

Download and install:

```sh
chmod +x memimpact
sudo mv memimpact /usr/local/bin/
```

---

## Usage Model

MemImpact operates in two complementary modes.

### Observer Mode (Shipped Binary)

The binary monitors an existing process ID.

```bash
memimpact <pid>
memimpact --name firefox
```

This mode is useful when:
- The process is already running  
- You want to attach to services  
- You are debugging long-lived processes  

### Command Mode (Shell Integration)

To measure a command from start to finish (like `time`), MemImpact is typically used with a small shell function you can add to your bashrc/zshrc:
```bash
memory() {
  "$@" &
  pid=$!
  memimpact --final $pid
  wait $pid
}
```

then

```
memory cargo build
memory python script.py
memory rg -c -o '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,3}' data.csv
```

#### Why a shell function?

MemImpact does not attempt to replace the shell’s job control, environment handling, or expansion logic.
Instead, it integrates with the shell to observe the process it launches.

This keeps MemImpact:
- simpler  
- shell-agnostic  
- free from command-parsing complexity  


---

## Accuracy & Sampling Model

MemImpact works by periodically reading memory statistics from: `/proc/<pid>/status` → VmRSS.  
This means:
- Memory is sampled, not continuously traced  
- Very short-lived spikes between samples may not be captured  
- Accuracy depends on the sampling rate (--hertz)  
- RSS reflects resident memory, not total system memory cost (e.g., swap, page cache attribution)  

MemImpact prioritizes low overhead and simplicity over profiler-level precision.  

For deep memory analysis, use tools like `/usr/bin/time -v` or `perf`.

### How MemImpact compares to other tools?

| Tool               | Purpose                                   | How MemImpact differs                                           |
| ------------------ | ----------------------------------------- | --------------------------------------------------------------- |
| `/usr/bin/time -v` | Reports resource usage after process exit | MemImpact can sample continuously and observe running processes |
| `psrecord`         | Python-based monitoring                   | MemImpact is a single static Rust binary with no runtime deps   |
| `valgrind massif`  | Heap profiling                            | Much deeper but slower and intrusive                            |
| `perf`             | System performance analysis               | Broader and more complex                                        |



---

## How It Works

1. Spawns your command as a child process
2. Monitors `/proc/<pid>/status` (and any child PIDs)
3. Tracks the peak RSS over the lifetime of the process
4. Prints a summary

No kernel modules, no ptrace, no dependencies — just reading `/proc`.

---

## Known Limitations

MemImpact reports sum of per-process RSS, which can exceed real physical RAM usage due to shared pages.
Values represent process footprint, not system-wide memory pressure.  

Resources utilization between samples can be missed.  

PID reuse after process exit can lead to incorrect attribution if the pid number recycling happens faster than one cycle duration.  

Behavior inside containers depends on PID namespace and cgroup configuration.  

---
## Contributing

Contributions, issues, and feature requests are welcome!
There is a CONTRIBUTING note aiming to help.  
Feel free to open a PR or an issue.

---
