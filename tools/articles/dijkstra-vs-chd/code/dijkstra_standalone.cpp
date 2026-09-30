#include <cstdint>
#include <functional>
#include <limits>
#include <queue>
#include <stdexcept>
#include <utility>
#include <vector>

using Distance = std::int64_t;
using Vertex = std::size_t;
constexpr Distance INF = std::numeric_limits<Distance>::max();

struct Edge {
    Vertex to;
    Distance weight;
};
using Graph = std::vector<std::vector<Edge>>;

std::vector<Distance> dijkstra(const Graph& graph, Vertex source) {
    if (source >= graph.size()) throw std::out_of_range("invalid source");
    for (const auto& row : graph) {
        for (const auto& e : row) {
            if (e.to >= graph.size()) throw std::out_of_range("invalid vertex");
            if (e.weight < 0 || e.weight == INF) {
                throw std::invalid_argument("invalid weight");
            }
        }
    }

    std::vector<Distance> dist(graph.size(), INF);
    using State = std::pair<Distance, Vertex>;  // 距离快照、顶点编号
    std::priority_queue<State, std::vector<State>, std::greater<State>> queue;

    dist[source] = 0;
    queue.emplace(0, source);

    while (!queue.empty()) {
        const auto [du, u] = queue.top();
        queue.pop();
        if (du != dist[u]) continue;  // 丢弃已被更短距离替代的记录

        for (const auto& e : graph[u]) {
            if (e.weight >= INF - du) {
                throw std::overflow_error("distance cannot be represented");
            }
            const Distance candidate = du + e.weight;
            if (candidate < dist[e.to]) {
                dist[e.to] = candidate;
                queue.emplace(candidate, e.to);
            }
        }
    }
    return dist;
}
