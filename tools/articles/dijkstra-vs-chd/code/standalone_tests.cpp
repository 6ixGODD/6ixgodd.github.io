// Independently test the exact standalone Dijkstra implementation shown in the article.
#include "dijkstra_standalone.cpp"
#include <iostream>
#include <random>

void require(bool ok) {
    if (!ok) throw std::runtime_error("standalone Dijkstra regression failed");
}
std::vector<Distance> reference(const Graph& g, Vertex source) {
    std::vector<Distance> d(g.size(), INF);
    d[source] = 0;
    for (std::size_t step = 1; step < g.size(); ++step) {
        auto next = d;
        for (Vertex u = 0; u < g.size(); ++u) if (d[u] != INF) {
            for (auto e : g[u]) next[e.to] = std::min(next[e.to], d[u] + e.weight);
        }
        if (next == d) break;
        d = std::move(next);
    }
    return d;
}
int main() {
    Graph sample = {{{1,10},{2,2}},{{3,1}},{{1,3},{3,10}},{},{}};
    require(dijkstra(sample,0) == std::vector<Distance>({0,5,2,6,INF}));
    Graph boundary = {{{1,INF-1}},{{1,0}}};
    require(dijkstra(boundary,0)[1] == INF-1);
    bool overflow = false;
    try { dijkstra(Graph{{{1,INF-1}},{{2,1}},{}},0); }
    catch (const std::overflow_error&) { overflow = true; }
    require(overflow);
    std::mt19937_64 rng(20260930);
    for (int trial=0;trial<2500;++trial) {
        std::size_t n=1+rng()%20;
        Graph g(n);
        for (Vertex u=0;u<n;++u) for (Vertex v=0;v<n;++v) {
            if (rng()%100<23) {
                g[u].push_back({v,static_cast<Distance>(rng()%15)});
                if (rng()%5==0) g[u].push_back({v,static_cast<Distance>(rng()%15)});
            }
        }
        Vertex source=rng()%n;
        require(dijkstra(g,source)==reference(g,source));
    }
    std::cout << "PASS: article Dijkstra snippet, 2500 seeded random multigraphs\n";
}
