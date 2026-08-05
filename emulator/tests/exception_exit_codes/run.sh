#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
source_dir=$repo_root/emulator/tests/exception_exit_codes
compiler=${RISCV_GCC:-riscv64-unknown-elf-gcc}
ziskemu=${ZISKEMU:-$repo_root/target/release/ziskemu}
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT

case ${1:-patched} in
  patched)
    cases=(ebreak:35 misaligned_branch:32 misaligned_jal:32 misaligned_jalr:32 clean_exit:0)
    ;;
  baseline)
    # The pre-change executable silently succeeds on transpile-time halts and does not enforce
    # IALIGN=32 for a no-C program.
    cases=(ebreak:0 misaligned_branch:0 misaligned_jal:0 misaligned_jalr:0 clean_exit:0)
    ;;
  *)
    echo "usage: $0 [baseline|patched]" >&2
    exit 2
    ;;
esac

if [[ ! -x $ziskemu ]]; then
  echo "ziskemu is not executable: $ziskemu" >&2
  exit 2
fi

for case_and_expected in "${cases[@]}"; do
  test_name=${case_and_expected%%:*}
  expected=${case_and_expected##*:}
  elf=$build_dir/$test_name.elf
  "$compiler" -nostdlib -nostartfiles -march=rv64im -mabi=lp64 \
    -Wl,--build-id=none -T "$source_dir/link.ld" -o "$elf" "$source_dir/$test_name.S"

  set +e
  "$ziskemu" --elf "$elf" --inputs /dev/null --max-steps 1000 >"$build_dir/$test_name.out" 2>&1
  actual=$?
  set -e

  if [[ $actual -ne $expected ]]; then
    echo "$test_name: expected exit $expected, got $actual" >&2
    sed -n '1,160p' "$build_dir/$test_name.out" >&2
    exit 1
  fi
  echo "$test_name: exit $actual"
done
