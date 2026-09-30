#!/bin/sh
# Compile into a disposable directory, never into the article's downloadable assets.
set -eu
cd "$(dirname "$0")"
bwww_test_dir=$(mktemp -d)
trap 'rm -rf "$bwww_test_dir"' EXIT HUP INT TERM
for test_source in tests.cpp standalone_tests.cpp; do
    clang++ -std=c++17 -O2 -Wall -Wextra -Wpedantic -Werror "$test_source" -o "$bwww_test_dir/test"
    "$bwww_test_dir/test"
    clang++ -std=c++17 -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer \
        -Wall -Wextra -Wpedantic -Werror "$test_source" -o "$bwww_test_dir/sanitized"
    "$bwww_test_dir/sanitized"
done
clang++ -std=c++17 -O2 -Wall -Wextra -Wpedantic -Werror example.cpp -o "$bwww_test_dir/example"
"$bwww_test_dir/example"
