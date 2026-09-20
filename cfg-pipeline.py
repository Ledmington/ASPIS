#!/usr/bin/env python3
"""Runs a C/C++ or Rust file through the same pass pipeline as aspis.sh, stage
by stage, and dumps the CFG (as .dot and .svg) of every function after each
stage. Useful to see exactly which pass introduces which basic blocks.

This is a readability-focused Python port of cfg-pipeline.sh. Behavior is
kept identical on purpose; cfg-pipeline.sh remains the canonical version and
is not being replaced.

A <source-file> ending in .rs is run through rustc + rust-annotation-bridge
instead of clang, matching aspis.sh's Rust front-end (see its comments for
why); this requires the rust-toolchain.toml-pinned rustc on PATH and both
rust-annotations/target/release/libaspis_annotations.rlib and
build/passes/libRUST_ANNOTATION_BRIDGE.so already built.
"""

from __future__ import annotations

import argparse
import glob
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

DIR = Path(__file__).resolve().parent

DUP_LIBS = {
    "eddi": "libEDDI.so",
    "seddi": "libSEDDI.so",
    "fdsc": "libFDSC.so",
    "reddi": "libREDDI.so",
    "none": None,
}

# technique -> (plugin .so, pass name)
CFC_PASSES = {
    "cfcss": ("libCFCSS.so", "cfcss-verify"),
    "rasm": ("libRASM.so", "rasm-verify"),
    "inter-rasm": ("libINTER_RASM.so", "rasm-verify"),
    "racfed": ("libRACFED.so", "racfed-verify"),
    "none": (None, None),
}


def parse_args():
    parser = argparse.ArgumentParser(
        prog="cfg-pipeline.py",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "source", help="C/C++/Rust source file to run through the pipeline"
    )
    parser.add_argument(
        "--llvm-bin",
        help="Directory containing clang/clang++/opt/llvm-link "
        "(default: read from testing/config/llvm.toml, falls back to $PATH)",
    )
    parser.add_argument(
        "--dup",
        default="eddi",
        choices=sorted(DUP_LIBS),
        help="Data duplication technique",
    )
    parser.add_argument(
        "--cfc",
        default="cfcss",
        choices=sorted(CFC_PASSES),
        help="Control-flow checking technique",
    )
    parser.add_argument(
        "--out-dir", default="./cfg-pipeline-out", help="Where stage output goes"
    )
    parser.add_argument(
        "--debug", action="store_true", help="Pass -debug to every opt invocation"
    )
    parser.add_argument(
        "--debug-only",
        action="append",
        default=[],
        metavar="TYPE",
        help="Pass -debug-only=TYPE to every opt invocation (e.g. eddi_verification); may be given more than once. "
        "Requires opt built with LLVM_ENABLE_ASSERTIONS=ON and the passes built without NDEBUG "
        "(e.g. -DCMAKE_CXX_FLAGS=-UNDEBUG), otherwise LLVM_DEBUG() is compiled out / -debug-only is unknown.",
    )
    return parser.parse_args()


def resolve_llvm_bin(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit)

    toml = DIR / "testing/config/llvm.toml"
    if toml.is_file():
        for line in toml.read_text().splitlines():
            line = line.strip()
            if line.startswith("#"):
                continue
            m = re.search(r'llvm_bin\s*=\s*"([^"]+)"', line)
            if m:
                return Path(m.group(1))

    clang = shutil.which("clang")
    if not clang:
        sys.exit(
            "Cannot determine --llvm-bin: no testing/config/llvm.toml and no 'clang' in PATH"
        )
    return Path(clang).parent


def require_executable(path: Path) -> None:
    if not os.access(path, os.X_OK):
        sys.exit(f"Cannot find/execute {path}")


class Pipeline:
    """Runs opt stages against a shared, continuously-overwritten .ll file,
    snapshotting the IR and CFGs after each one."""

    def __init__(self, opt: Path, out_dir: Path, llvm_debug_args: list[str]):
        self.opt = opt
        self.out_dir = out_dir
        self.llvm_debug_args = llvm_debug_args
        self.cur_ll = out_dir / "current.ll"
        self.stage_num = 0

    def generate_cfg(self, ll_file: Path, dot_dir: Path) -> None:
        """Renders every function's CFG in ll_file as .dot + .svg into dot_dir."""
        dot_dir.mkdir(parents=True, exist_ok=True)
        ll_file = ll_file.resolve()
        with tempfile.TemporaryDirectory() as tmp:
            # Not checked: opt can legitimately fail here yet still have
            # emitted .dot files for the functions it got through first.
            subprocess.run(
                [self.opt, "--passes=dot-cfg", "-disable-output", str(ll_file)],
                cwd=tmp,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            for dotf in glob.glob(os.path.join(tmp, ".*.dot")):
                base = Path(dotf).name[1 : -len(".dot")]
                shutil.copy(dotf, dot_dir / f"{base}.dot")
                subprocess.run(
                    ["dot", "-Tsvg", dotf, "-o", str(dot_dir / f"{base}.svg")],
                    check=True,
                )

    def snapshot(self, tag: str) -> None:
        shutil.copy(self.cur_ll, self.out_dir / f"{tag}.ll")
        self.generate_cfg(self.cur_ll, self.out_dir / f"{tag}_cfg")

    def run_stage(self, label: str, command: list) -> None:
        tag = f"{self.stage_num:02d}_{label}"
        print(f"=== [{tag}] {' '.join(str(c) for c in command)} ===")
        subprocess.run(command, check=True)
        self.snapshot(tag)
        self.stage_num += 1

    def opt_pass_cmd(self, passes: str) -> list:
        return [
            self.opt,
            f"--passes={passes}",
            str(self.cur_ll),
            "-o",
            str(self.cur_ll),
            "-S",
            *self.llvm_debug_args,
        ]

    def opt_plugin_pass_cmd(self, lib: str, passes: str) -> list:
        return [
            self.opt,
            f"-load-pass-plugin={DIR}/build/passes/{lib}",
            f"--passes={passes}",
            str(self.cur_ll),
            "-o",
            str(self.cur_ll),
            "-S",
            *self.llvm_debug_args,
        ]


def run_rust_frontend(
    rustc: str, src: Path, rust_annotations_rlib: Path, llvm_dis: Path, cur_ll: Path
) -> None:
    """Mirrors aspis.sh's Rust front-end: rustc emits one codegen-unit
    bitcode file (-C codegen-units=1 guarantees exactly one), which becomes
    the starting IR in place of clang's output. -C save-temps keeps that
    bitcode on disk after rustc exits so it can be picked up here."""
    filename = src.stem
    src_abs = src.resolve()
    with tempfile.TemporaryDirectory() as tmp:
        with open(os.path.join(tmp, "rust_link_cmd.sh"), "w") as link_cmd:
            subprocess.run(
                [
                    rustc,
                    "--edition",
                    "2021",
                    "-C",
                    "debuginfo=2",
                    "-C",
                    "codegen-units=1",
                    "-C",
                    "save-temps",
                    "--crate-type=bin",
                    "--extern",
                    f"aspis_annotations={rust_annotations_rlib}",
                    "--print",
                    "link-args",
                    "-o",
                    f"{filename}.unhardened.out",
                    str(src_abs),
                ],
                cwd=tmp,
                stdout=link_cmd,
                check=True,
            )
        cgu_bc = glob.glob(os.path.join(tmp, "*-cgu.0.rcgu.bc"))
        if not cgu_bc:
            sys.exit(f"Could not locate rustc's codegen-unit bitcode output in {tmp}")
        subprocess.run([llvm_dis, cgu_bc[0], "-o", str(cur_ll)], check=True)


def main() -> None:
    args = parse_args()
    src = Path(args.source)
    if not src.is_file():
        sys.exit(f"No such file: {src}")

    llvm_bin = resolve_llvm_bin(args.llvm_bin)
    clang, clangxx, opt, llvm_dis = (
        llvm_bin / name for name in ("clang", "clang++", "opt", "llvm-dis")
    )

    rust_input = src.suffix == ".rs"
    rustc = rust_annotations_rlib = bridge_lib = frontend = None

    if rust_input:
        rustc = shutil.which("rustc")
        if not rustc:
            sys.exit("Cannot find 'rustc' in PATH")
        require_executable(opt)
        require_executable(llvm_dis)

        rust_annotations_rlib = (
            DIR / "rust-annotations/target/release/libaspis_annotations.rlib"
        )
        if not rust_annotations_rlib.is_file():
            sys.exit(
                f"Missing {rust_annotations_rlib} (build it first: cmake --build build)"
            )

        bridge_lib = DIR / "build/passes/libRUST_ANNOTATION_BRIDGE.so"
        if not bridge_lib.is_file():
            sys.exit(f"Missing {bridge_lib} (build it first: cmake --build build)")
    else:
        for tool in (clang, clangxx, opt):
            require_executable(tool)
        frontend = clangxx if src.suffix in (".cpp", ".cc", ".cxx") else clang

    if not shutil.which("dot"):
        sys.exit("Graphviz 'dot' not found in PATH")

    dup_lib = DUP_LIBS[args.dup]
    cfc_lib, cfc_pass = CFC_PASSES[args.cfc]

    out_dir = Path(args.out_dir)
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)

    llvm_debug_args = (["-debug"] if args.debug else []) + [
        f"-debug-only={t}" for t in args.debug_only
    ]

    pipeline = Pipeline(opt, out_dir, llvm_debug_args)

    print("== Frontend ==")
    if rust_input:
        run_rust_frontend(rustc, src, rust_annotations_rlib, llvm_dis, pipeline.cur_ll)
    else:
        subprocess.run(
            [
                frontend,
                str(src),
                "-S",
                "-emit-llvm",
                "-O0",
                "-Xclang",
                "-disable-O0-optnone",
                "-o",
                str(pipeline.cur_ll),
            ],
            check=True,
        )
    pipeline.snapshot("00_frontend")
    pipeline.stage_num = 1

    if rust_input:
        pipeline.run_stage(
            "rust-annotation-bridge",
            pipeline.opt_plugin_pass_cmd(
                "libRUST_ANNOTATION_BRIDGE.so", "rust-annotation-bridge"
            ),
        )

    pipeline.run_stage("lower-switch", pipeline.opt_pass_cmd("lower-switch"))
    pipeline.run_stage(
        "func-ret-to-ref", pipeline.opt_plugin_pass_cmd("libEDDI.so", "func-ret-to-ref")
    )

    if dup_lib:
        pipeline.run_stage(
            f"{args.dup}-verify", pipeline.opt_plugin_pass_cmd(dup_lib, "eddi-verify")
        )

    pipeline.run_stage("simplifycfg", pipeline.opt_pass_cmd("simplifycfg"))

    if cfc_lib:
        pipeline.run_stage(cfc_pass, pipeline.opt_plugin_pass_cmd(cfc_lib, cfc_pass))

    if dup_lib:
        pipeline.run_stage(
            "duplicate-globals",
            pipeline.opt_plugin_pass_cmd("libEDDI.so", "duplicate-globals"),
        )

    print()
    print(f"Done. Stage-by-stage IR and CFGs (dot + svg) are in: {out_dir}")
    print(
        "Each NN_<stage>_cfg/ directory holds one .dot/.svg pair per function as it looked right after that stage."
    )


if __name__ == "__main__":
    main()
