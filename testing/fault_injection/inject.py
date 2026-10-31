#!/usr/bin/env python3

import argparse
from pathlib import Path
import os
import sys
import shutil
import subprocess
import shlex
from collections.abc import Sequence

# lldb exit codes set by the breakpoint commands on the handlers
DATA_EXIT_CODE = 10
SIG_EXIT_CODE = 11


def run_command(
    command: Sequence[str], timeout: float = 5.0, check: bool = False
) -> subprocess.CompletedProcess[str]:
    print(shlex.join(command))
    return subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        errors="replace",
        timeout=timeout,
        check=check,
    )


def classify_result(result: subprocess.CompletedProcess[str]) -> str:
    if result.returncode == DATA_EXIT_CODE:
        return "data corruption detected"
    if result.returncode == SIG_EXIT_CODE:
        return "signature mismatch detected"
    if "stop reason = signal" in result.stdout:
        return "crash"
    if "exited with status = " in result.stdout:
        return "not detected"
    return "unknown"


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="fault_injection",
        description="Runs a fault injection campaign against a given executable file",
    )
    parser.add_argument(
        "-f",
        "--file",
        type=Path,
        required=True,
        help="the executable file to inject faults into",
    )
    parser.add_argument(
        "-t",
        "--timeout",
        type=float,
        default=5.0,
        help="seconds after which a run of the executable is considered hung",
    )
    parser.add_argument(
        "--check-detection",
        action="store_true",
        help="exit with failure if no DataCorruption or SigMismatch is detected",
    )

    args = parser.parse_args()

    if not args.file.is_file():
        parser.error(f"{args.file} does not exist or is not a regular file")

    if not os.access(args.file, os.X_OK):
        parser.error(f"{args.file} is not executable")

    lldb_bin = shutil.which("lldb")
    if lldb_bin is None:
        parser.error("No lldb found.")

    nm_bin = shutil.which("nm")
    if nm_bin is None:
        parser.error("No nm found.")

    file = os.path.abspath(args.file)

    mangled_symbols = {}
    output = run_command([nm_bin, "--defined-only", file], check=True).stdout
    for line in output.split("\n"):
        line = line.strip()
        if len(line) == 0:
            continue
        parts = line.split(" ")
        address = parts[0]
        mangled_name = " ".join(parts[2:])
        mangled_symbols[address] = mangled_name

    demangled_symbols = {}
    output = run_command([nm_bin, "-C", "--defined-only", file], check=True).stdout
    for line in output.split("\n"):
        line = line.strip()
        if len(line) == 0:
            continue
        parts = line.split(" ")
        address = parts[0]
        demangled_name = " ".join(parts[2:])
        demangled_symbols[address] = demangled_name

    demangled_to_mangled_symbols = {}
    for address, demangled_name in demangled_symbols.items():
        if address not in mangled_symbols:
            continue
        demangled_to_mangled_symbols[demangled_name] = mangled_symbols[address]

    mangled_main = demangled_to_mangled_symbols["add::main"]
    mangled_A = demangled_to_mangled_symbols["add::A"]

    command = [
        lldb_bin,
        "--batch",
        "--no-lldbinit",
        "-o",
        "settings set target.disable-aslr false",
        "-o",
        "breakpoint set -n DataCorruption_Handler",
        "-o",
        f"breakpoint command add -o 'quit {DATA_EXIT_CODE}' 1",
        "-o",
        "breakpoint set -n SigMismatch_Handler",
        "-o",
        f"breakpoint command add -o 'quit {SIG_EXIT_CODE}' 2",
        "-o",
        f"breakpoint set -n {mangled_main}",
        "-o",
        "run",
        "-o",
        f"expression -l c -- *(int*)&{mangled_A} = 99",
        "-o",
        "continue",
        "-k",
        "quit",
        "--",
        file,
    ]

    try:
        result = run_command(command, timeout=args.timeout)
        outcome = classify_result(result)
        print(result.stdout)
    except subprocess.TimeoutExpired:
        outcome = "hang"

    print(f"outcome : {outcome}")

    detected = ("data corruption detected", "signature mismatch detected")
    if args.check_detection and outcome not in detected:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
