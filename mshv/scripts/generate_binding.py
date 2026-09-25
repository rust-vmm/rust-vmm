#!/usr/bin/env python3

import argparse
import logging
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from shutil import rmtree, which

crate_dir = Path(__file__).resolve().parent.parent

mshv_header_files = [
    "include/linux/mshv.h",
]

hv_header_files = [
    "hvgdk_mini.h",
    "hvgdk.h",
    "hvhdk_mini.h",
    "hvhdk.h",
]


def check_installed(cmd):
    return which(cmd) is not None


def install_kernel_headers(kernel_src_path, arch):
    kernel_hdr_path = tempfile.mkdtemp(prefix="linux")
    logging.debug(f"Installing kernel headers at {kernel_hdr_path}")
    subprocess.run(
        [
            "make",
            "headers_install",
            f"ARCH={arch}",
            f"INSTALL_HDR_PATH={kernel_hdr_path}",
            "-C",
            kernel_src_path,
        ],
        capture_output=True,
        check=True,
    )
    return kernel_hdr_path


def generate_unified_mshv_headers(kernel_hdr_path, hv_hdrs_path):
    logging.debug("Start generating unified header file")

    with open(f"{kernel_hdr_path}/combined_mshv.h", "w") as fp:
        fp.write("typedef _Bool bool;\n")
        data = ""

        for header in hv_header_files:
            header_path = f"{hv_hdrs_path}/{header}"

            with open(header_path, "r") as f:
                data += f.read()

        for header in mshv_header_files:
            header_path = f"{kernel_hdr_path}/{header}"

            with open(header_path, "r") as f:
                data += f.read()

        for header_name in hv_header_files:
            regexp = f".*{header_name}.*"
            data = re.sub(regexp, "", data, flags=re.M)

        data = re.sub(r"BIT\(([A-Z_0-9]+)\)", "(1 << (\\1))", data, flags=re.M)

        fp.write(data)

    logging.debug("Done generating unified header file")


def run_bindgen(kernel_hdr_path, output_file, bindgen_args, clang_target):
    cmd = f"""
    bindgen {bindgen_args} \
    {kernel_hdr_path}/combined_mshv.h -- --target={clang_target} -I {kernel_hdr_path}/include > {output_file}
    """
    logging.debug("Running bindgen: %s", cmd)

    subprocess.check_call(cmd, shell=True)
    logging.debug("Cleaning up installed header files")
    rmtree(kernel_hdr_path)


def update_bindings_comment(bindings_file):
    comment_lines = [
        "/*\n",
        " * Kernel (uapi) headers used for these bindings are as follows:\n",
        " */\n"
    ]
    comment_lines[2:2] = map(lambda s: f" * {s}\n", mshv_header_files)

    with open(bindings_file, "r") as f:
        lines = f.readlines()

    lines[1:1] = comment_lines

    with open(bindings_file, "w") as f:
        f.write("".join(lines))


def get_clang_target(arch):
    if arch == "x86":
        return "x86_64-linux-gnu"

    return "aarch64-linux-gnu"


def get_arch_location(arch):
    if arch == "x86":
        return "x86_64"

    return arch


def main(args):
    bindgen = check_installed("bindgen")
    if not bindgen:
        logging.error("Please install bindgen utility.")
        return -1

    make = check_installed("make")
    if not make:
        logging.error("Please install make.")
        return -1

    kernel_hdr_path = install_kernel_headers(args.kernel_src_path, args.arch)
    generate_unified_mshv_headers(kernel_hdr_path, args.hv_hdrs_path)

    bindgen_args = "--no-doc-comments --with-derive-default "

    bindgen_args += args.bindgen_args
    arch_location = get_arch_location(args.arch)
    clang_target = get_clang_target(args.arch)
    output_file = f"{args.output}/{arch_location}/bindings.rs"

    run_bindgen(kernel_hdr_path, output_file, bindgen_args, clang_target)
    update_bindings_comment(output_file)

    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generate Rust bindings from Hyper-V headers from Linux kernel",
        epilog="example:\n"
        "./generate_binding.py  -k /opt/linux-dom0\n"
        "./generate_binding.py  -k /opt/linux-dom0 --arch x86\n"
        "./generate_binding.py  -k /opt/linux-dom0 --bindgen=\"--with-derive-eq --with-derive-partialeq --with-derive-ord\"",
        formatter_class=argparse.RawTextHelpFormatter
    )

    parser.add_argument(
        "--kernel",
        "-k",
        type=str,
        dest="kernel_src_path",
        required=True,
        help="Linux kernel source path",
    )

    parser.add_argument(
        "--headers",
        "-H",
        type=str,
        dest="hv_hdrs_path",
        default=str(crate_dir.joinpath("hv-headers")),
        help="Hypervisor headers path",
    )

    parser.add_argument(
        "--output",
        "-o",
        type=str,
        dest="output",
        default=str(crate_dir.joinpath("mshv-bindings/src")),
        help="Directory to store bindgen.rs",
    )

    parser.add_argument(
        "--bindgen",
        "-b",
        type=str,
        dest="bindgen_args",
        default="",
        help="Additional bindgen arguments",
    )

    parser.add_argument(
        "--log-level",
        "-l",
        dest="log_level",
        choices=["info", "debug", "error"],
        default="info",
        help="Log level for logging (default: %(default)s)",
    )

    parser.add_argument(
        "--arch",
        "-a",
        dest="arch",
        choices=["x86", "arm64"],
        default="x86",
        help="Architecture for binding generation (default: %(default)s)",
    )

    log_level = {"info": logging.INFO, "debug": logging.DEBUG, "error": logging.ERROR}

    args = parser.parse_args()
    logging.getLogger().setLevel(log_level.get(args.log_level, logging.INFO))

    if not Path(args.kernel_src_path).is_dir():
        logging.error("Invalid kernel path:", args.kernel_src_path)
        sys.exit(-1)

    res = main(args)
    sys.exit(res)
