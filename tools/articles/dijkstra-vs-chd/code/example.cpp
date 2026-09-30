#include "shortest_paths.hpp"
#include <iostream>

int main() {
    paths::Graph graph(5);
    graph[0] = {{1, 10, 1}, {2, 2, 2}};
    graph[2] = {{1, 3, 3}, {3, 10, 4}};
    graph[1] = {{3, 1, 5}};
    auto distances = paths::dijkstra(graph, 0);
    for (std::size_t v = 0; v < distances.size(); ++v) {
        std::cout << v << ": ";
        if (distances[v] == paths::INF) std::cout << "INF\n";
        else std::cout << distances[v] << '\n';
    }
}
