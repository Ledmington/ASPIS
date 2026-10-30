#!/usr/bin/env python3

import argparse
from pathlib import Path
import shutil
import tempfile
import subprocess
import random
import os
import sys
from collections import Counter

from elftools.elf.elffile import ELFFile

DATA_SENTINEL = "ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler"
SIG_SENTINEL = "ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler"


def flip_random_bit(file_path, section_name):
    with open(file_path, "r+b") as f:
        elf = ELFFile(f)
        section = elf.get_section_by_name(section_name)

        if section is None:
            raise ValueError(f"Section not found: {section_name}")

        offset = section["sh_offset"]
        size = section["sh_size"]

        if size == 0:
            raise ValueError(f"Section is empty: {section_name}")

        # Choose a random byte and bit
        byte_offset = offset + random.randrange(size)
        bit_index = random.randrange(8)

        # Read the byte
        f.seek(byte_offset)
        original_byte = f.read(1)[0]

        # Flip exactly one bit
        modified_byte = original_byte ^ (1 << bit_index)

        # Write the modified byte
        f.seek(byte_offset)
        f.write(bytes([modified_byte]))

        return byte_offset, bit_index


def classify(output, exit_code, timed_out):
    # The handlers exit with a non-zero code, so check them before crashes.
    if DATA_SENTINEL in output:
        return "data_corruption"
    if SIG_SENTINEL in output:
        return "sig_mismatch"
    if timed_out or exit_code != 0:
        return "crashed"
    return "undetected"


def run_fault_injection_campaign(file, sections, seed, num_attempts, timeout):

    print(f"File          : {file}")
    print(f"Sections      : {sections}")
    print(f"Seed          : {seed}")
    print(f"Num. Attempts : {num_attempts}")
    print(f"Timeout       : {timeout}s")

    random.seed(seed)
    counts = Counter()

    for _ in range(num_attempts):

        with tempfile.NamedTemporaryFile(
            prefix="aspis_fault_injection_",
            delete=False,
            delete_on_close=False,
        ) as tmp:
            temp_path = Path(tmp.name)

        try:
            shutil.copy2(file, temp_path)
            flip_random_bit(temp_path, random.choice(sections))

            timed_out = False
            try:
                result = subprocess.run(
                    [str(temp_path)],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    errors="replace",
                    timeout=timeout,
                )
                output = result.stdout
                exit_code = result.returncode
            except subprocess.TimeoutExpired as e:
                timed_out = True
                output = e.stdout or ""
                if isinstance(output, bytes):
                    output = output.decode(errors="replace")
                exit_code = None

            counts[classify(output, exit_code, timed_out)] += 1

        finally:
            temp_path.unlink(missing_ok=True)

    return counts


def print_summary(counts, num_attempts):
    print()
    for outcome in ["data_corruption", "sig_mismatch", "crashed", "undetected"]:
        n = counts[outcome]
        print(f"{outcome:<16}: {n:4d} ({100 * n / num_attempts:5.1f}%)")


def main():
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
        "--sections",
        nargs="+",
        default=[".data", ".rodata"],
        help="names of sections to modify",
    )
    parser.add_argument(
        "-s", "--seed", type=int, default=42, help="the seed for random fault injection"
    )
    parser.add_argument(
        "-n",
        "--num-attempts",
        type=int,
        default=100,
        help="number of random faults to inject",
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

    with args.file.open("rb") as f:
        elf = ELFFile(f)

        file_sections = {sec.name: sec for sec in elf.iter_sections()}

        for section in args.sections:
            if section not in file_sections:
                parser.error(
                    f"Executable {args.file} does not contain a section named {section}"
                )
            # NOBITS sections (e.g. .bss) occupy no bytes in the file.
            if file_sections[section]["sh_type"] == "SHT_NOBITS":
                parser.error(f"Section {section} has no bytes in the file")

    if args.num_attempts <= 0:
        parser.error("Number of attempts must be >0.")

    counts = run_fault_injection_campaign(
        args.file, args.sections, args.seed, args.num_attempts, args.timeout
    )
    print_summary(counts, args.num_attempts)

    if args.check_detection and not (
        counts["data_corruption"] or counts["sig_mismatch"]
    ):
        print("No fault was detected", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
