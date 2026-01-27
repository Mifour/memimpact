# MemImpact

**MemImpact** is a lightweight, CLI tool written in Rust that measures the **maximum memory usage** of any terminal command — including all of its child processes.  
It works similarly to the classic Unix `time` command, but focuses specifically on memory.

Perfect for easy benchmarking, profiling, or simply understanding how much RAM your commands actually consume.

---

## Features

-  Measures peak memory usage (RSS) of any command  
-  Includes memory from **child processes**  
-  Works on any Linux (uses `/proc/<pid>/status`)  
-  Fast, zero external dependencies, written in pure Rust std  
-  Works as a direct command or with an optional shell wrapper  
-  Easy to install
-  Easy to use, just add `memimpact` in front of your command

---
## Limitations

- There is no support for MacOS and Windows. And it is not planned because their memory management is different.
- Very short-lived processes can cause issues. Memimpact has been tested to run successfuly down to 1/10_000th second.

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

### **Optional: Add a `memimpact` shell wrapper**

If you'd like a `time`-like interface, add this to your `.bashrc` or `.zshrc`:

```sh
memory() {
  "$@" &
  pid=$!
  /path/to/memimpact --final $pid # &
  wait $pid
}
```

---

## Usage

### help
```sh
➜ ./memimpact --help            
Memimpact -- measure the memory impact of any PID and its children processes.
Version: 0.0.8
Usage: memimpact <options> <pid>
Options:
--hertz int, the desired number of iterations per second
--output-file str, the file path where to write the output (stdout if absent)
--name str, sum up the memory usage of all processes with this name (disable the <pid> argument)
Flags:
--final, display only 1 line with the max value
```

### Basic usage

#### call the binary directly
  
Providing a pid

```sh
./memimpact 115404
Tracking memory usage of PID 115404 (spotify)
PID 115404 (spotify): current 411MB, max 411MB
PID 115404 (spotify): current 406MB, max 411MB
...
```
  
Providing a name
```sh
./memimpact --name firefox
Tracking memory usage of PID 5666 (firefox)
PID 5666 (firefox): current 2GB, max 2GB
PID 5666 (firefox): current 2GB, max 2GB
...
```
#### Using the optional shell wrapper
Example:

```sh
memory rg -c -o '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,3}' data.csv
```

Output example:

```
[2] 183480
39799522
[2]  + 183480 done       "$@"
PID 183480 (rg): max 3GB
```

---

## How It Works

1. Spawns your command as a child process
2. Monitors `/proc/<pid>/status` (and any child PIDs)
3. Tracks the peak RSS over the lifetime of the process
4. Prints a summary

No kernel modules, no ptrace, no dependencies — just reading `/proc`.

---
## Contributing

Contributions, issues, and feature requests are welcome!
There is a CONTRIBUTING note aiming to help.  
Feel free to open a PR or issue.

---

## License

MIT License

Copyright (c) 2026, Mifour

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---
