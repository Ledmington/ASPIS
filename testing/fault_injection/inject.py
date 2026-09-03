#!/usr/bin/env python3

import argparse
import random
import re
import shutil
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path

import capstone

FAULT_INJECTION_DIR = Path(__file__).resolve().parent
TESTING_DIR = FAULT_INJECTION_DIR.parent
ASPIS_ROOT = TESTING_DIR.parent
ASPIS_SH = ASPIS_ROOT / "aspis.sh"
DEFAULT_SOURCE = TESTING_DIR / "tests" / "rust" / "control_flow" / "loop_exit.rs"
DEFAULT_BUILD_DIR = FAULT_INJECTION_DIR / "build"

DATA_SENTINEL = "ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler"
SIG_SENTINEL = "ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler"


def default_llvm_bin() -> str:
    with open(TESTING_DIR / "config" / "llvm.toml", "rb") as f:
        return tomllib.load(f)["llvm_bin"]


def compile_with_aspis(
    source: Path, out_name: str, options: list[str], llvm_bin: str, build_dir: Path
) -> Path:
    build_dir.mkdir(parents=True, exist_ok=True)
    out_path = build_dir / f"{out_name}.out"
    command = [
        str(ASPIS_SH),
        "--llvm-bin",
        llvm_bin,
        *options,
        str(source),
        "-o",
        f"{out_name}.out",
        "--build-dir",
        str(build_dir),
    ]
    result = subprocess.run(command, cwd=ASPIS_ROOT, text=True, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(
            f"ASPIS compilation failed:\n{result.stdout}\n{result.stderr}"
        )
    if not out_path.exists():
        raise RuntimeError(f"ASPIS reported success but {out_path} was not produced")
    return out_path


_SECTION_RE = re.compile(
    r"^\s*\[\s*\d+\]\s+(?P<name>\S+)\s+\S+\s+(?P<addr>[0-9a-fA-F]+)\s+(?P<off>[0-9a-fA-F]+)\s+(?P<size>[0-9a-fA-F]+)\s"
)
_SYMBOL_RE = re.compile(
    r"^\s*\d+:\s+(?P<value>[0-9a-fA-F]+)\s+(?P<size>\d+)\s+(?P<type>\S+)\s+\S+\s+\S+\s+\S+\s+(?P<name>\S+)"
)


def text_section_address_offset(binary: Path) -> tuple[int, int]:
    sections = subprocess.run(
        ["readelf", "-W", "-S", str(binary)], text=True, capture_output=True, check=True
    ).stdout
    for line in sections.splitlines():
        m = _SECTION_RE.match(line)
        if m and m.group("name") == ".text":
            return int(m.group("addr"), 16), int(m.group("off"), 16)
    raise RuntimeError(f"could not find .text section in {binary}")


def function_byte_range(
    binary: Path, func_name: str, text_addr: int, text_off: int
) -> tuple[int, int, int]:
    symbols = subprocess.run(
        ["readelf", "-W", "-s", str(binary)], text=True, capture_output=True, check=True
    ).stdout
    for line in symbols.splitlines():
        m = _SYMBOL_RE.match(line)
        if m and m.group("type") == "FUNC" and m.group("name") == func_name:
            value = int(m.group("value"), 16)
            size = int(m.group("size"))
            if size == 0:
                raise RuntimeError(
                    f"symbol {func_name!r} has size 0, cannot pick an injection offset"
                )
            file_offset = text_off + (value - text_addr)
            return file_offset, size, value

    raise RuntimeError(
        f"could not find FUNC symbol {func_name!r} in {binary} (is it stripped?)"
    )


def operand_biased_offsets(
    binary: Path, file_offset: int, size: int, addr: int
) -> list[int]:
    code = binary.read_bytes()[file_offset : file_offset + size]
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    md.detail = True

    candidates: list[int] = []
    for insn in md.disasm(code, addr):
        insn_file_offset = file_offset + (insn.address - addr)
        enc = insn.encoding
        if enc.imm_size:
            candidates.extend(
                range(
                    insn_file_offset + enc.imm_offset,
                    insn_file_offset + enc.imm_offset + enc.imm_size,
                )
            )
        if enc.disp_size:
            candidates.extend(
                range(
                    insn_file_offset + enc.disp_offset,
                    insn_file_offset + enc.disp_offset + enc.disp_size,
                )
            )

    return candidates


def flip_random_bit_at(
    data: bytearray, candidates: list[int], rng: random.Random
) -> tuple[int, int]:
    byte_index = rng.choice(candidates)
    bit_index = rng.randrange(8)
    data[byte_index] ^= 1 << bit_index
    return byte_index, bit_index


def classify(stdout: str, returncode: int, golden_stdout: str, timed_out: bool) -> str:
    caught_data = DATA_SENTINEL in stdout
    caught_sig = SIG_SENTINEL in stdout
    if caught_data and caught_sig:
        return "caught_both"
    if caught_data:
        return "caught_data"
    if caught_sig:
        return "caught_sig"
    if timed_out:
        return "hang_undetected"
    if returncode < 0:
        return "crashed_undetected"
    if stdout == golden_stdout:
        return "silent_no_effect"
    return "silent_wrong_output"


def run_campaign(
    binary: Path, func_name: str, trials: int, seed: int, timeout: float, work_dir: Path
) -> Counter:
    text_addr, text_off = text_section_address_offset(binary)
    offset, size, addr = function_byte_range(binary, func_name, text_addr, text_off)

    try:
        candidates = operand_biased_offsets(binary, offset, size, addr)
    except ModuleNotFoundError:
        raise RuntimeError(
            "capstone is required for operand-biased fault injection: "
            "pip install -r testing/fault_injection/requirements.txt"
        )
    if not candidates:
        raise RuntimeError(
            f"{func_name!r} has no immediate/displacement operand bytes to inject into"
        )

    print(
        f"  injecting into {func_name!r}: file offset 0x{offset:x}, size {size} bytes, "
        f"{len(candidates)} operand-biased candidate bytes"
    )

    original = bytearray(binary.read_bytes())
    golden = subprocess.run(
        [str(binary)], text=True, errors="replace", capture_output=True, timeout=timeout
    )
    golden_stdout = golden.stdout

    rng = random.Random(seed)
    work_dir.mkdir(parents=True, exist_ok=True)
    patched_path = work_dir / f"{binary.stem}.patched"

    counts: Counter = Counter()
    for _ in range(trials):
        patched = bytearray(original)
        flip_random_bit_at(patched, candidates, rng)
        patched_path.write_bytes(patched)
        patched_path.chmod(0o755)

        timed_out = False
        try:
            result = subprocess.run(
                [str(patched_path)],
                text=True,
                errors="replace",
                capture_output=True,
                timeout=timeout,
            )
            stdout, returncode = result.stdout, result.returncode
        except subprocess.TimeoutExpired as e:
            timed_out = True
            stdout, returncode = (e.stdout or ""), -1

        outcome = classify(stdout, returncode, golden_stdout, timed_out)
        counts[outcome] += 1

    patched_path.unlink(missing_ok=True)
    return counts


def print_summary(label: str, counts: Counter, trials: int) -> None:
    print(f"\n{label} ({trials} trials):")
    for category in [
        "caught_data",
        "caught_sig",
        "caught_both",
        "crashed_undetected",
        "hang_undetected",
        "silent_wrong_output",
        "silent_no_effect",
    ]:
        n = counts.get(category, 0)
        if n:
            print(f"  {category:<22} {n:4d}  ({100 * n / trials:5.1f}%)")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument(
        "--llvm-bin",
        default=None,
        help="defaults to testing/config/llvm.toml's llvm_bin",
    )
    parser.add_argument("--func", default="main", help="function to inject faults into")
    parser.add_argument("--data-technique", default="--eddi")
    parser.add_argument("--cfc-technique", default="--cfcss")
    parser.add_argument("--trials", type=int, default=100)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--timeout", type=float, default=5.0)
    parser.add_argument("--build-dir", type=Path, default=DEFAULT_BUILD_DIR)
    args = parser.parse_args()

    for name, value in vars(args).items():
        print(f"  {name}: {value}")

    llvm_bin = args.llvm_bin or default_llvm_bin()
    if shutil.which("clang", path=llvm_bin) is None:
        print(
            f"warning: clang not found under --llvm-bin {llvm_bin!r}",
            file=sys.stderr,
        )

    print(f"Compiling hardened build ({args.data_technique} {args.cfc_technique})...")
    hardened = compile_with_aspis(
        args.source,
        "hardened",
        [args.data_technique, args.cfc_technique],
        llvm_bin,
        args.build_dir,
    )
    hardened_counts = run_campaign(
        hardened,
        args.func,
        args.trials,
        args.seed,
        args.timeout,
        args.build_dir / "hardened_trials",
    )
    print_summary(
        f"hardened [{args.data_technique} {args.cfc_technique}]",
        hardened_counts,
        args.trials,
    )

    return 0


if __name__ == "__main__":
    sys.exit(main())
