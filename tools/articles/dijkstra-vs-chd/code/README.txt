C++ reproduction (requires clang++ with C++17 and sanitizers):
  sh tools/articles/dijkstra-vs-chd/code/verify.sh

- shortest_paths.hpp: complete Dijkstra + ONE C-HD local search, not full C-HD.
- tests.cpp: directed regressions and 2,500 random simple directed graphs.
- dijkstra_standalone.cpp: exact distance-only implementation printed in the article;
  no main function, tested by standalone_tests.cpp.
- standalone_tests.cpp: 2,500 random multigraphs + integer-boundary checks.
- example.cpp: prints 0, 5, 2, 6, INF.

DijkstraStep.lean contains two LOCAL lemmas and has NOT been compiled locally.
CheckCHD.lean must run inside the original pinned C-HD proof project.
Neither file proves this C++ implementation is a formal refinement of C-HD.

Figure reproduction:
  uv run tools/articles/dijkstra-vs-chd/generate_figures.py (macOS; requires sips)
SVG text uses the site's Times New Roman, Times, serif font stack and browser
fallback for Chinese. The diagrams are not performance benchmark results.
