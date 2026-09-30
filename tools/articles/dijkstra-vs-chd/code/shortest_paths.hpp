#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <limits>
#include <stdexcept>
#include <tuple>
#include <utility>
#include <vector>
#include <queue>

namespace paths {
using Vertex = std::size_t;
using Distance = std::int64_t;
inline constexpr Distance INF = std::numeric_limits<Distance>::max();
inline constexpr std::size_t NONE = std::numeric_limits<std::size_t>::max();

struct Edge {
    Vertex to;
    Distance weight;
    std::size_t id;  // Positive, globally unique; zero denotes no last edge.
};
using Graph = std::vector<std::vector<Edge>>;

inline Distance checked_add(Distance a, Distance b) {
    if (a < 0 || b < 0 || a == INF || b >= INF - a) {
        throw std::overflow_error("distance is outside [0, INT64_MAX)");
    }
    return a + b;
}

inline void validate_graph(const Graph& graph, Vertex source) {
    if (source >= graph.size()) throw std::out_of_range("invalid source");
    for (const auto& row : graph) {
        for (const auto& e : row) {
            if (e.to >= graph.size()) throw std::out_of_range("invalid vertex");
            if (e.weight < 0 || e.weight == INF) {
                throw std::invalid_argument("expected finite nonnegative weight");
            }
        }
    }
}

// Complete distance-only SSSP. Parallel edges, zero weights, and self-loops are allowed.
inline std::vector<Distance> dijkstra(const Graph& graph, Vertex source) {
    validate_graph(graph, source);
    std::vector<Distance> dist(graph.size(), INF);
    using State = std::pair<Distance, Vertex>;
    std::priority_queue<State, std::vector<State>, std::greater<State>> queue;
    dist[source] = 0;
    queue.emplace(0, source);
    while (!queue.empty()) {
        const auto [du, u] = queue.top();
        queue.pop();
        if (du != dist[u]) continue;
        for (const auto& e : graph[u]) {
            const Distance candidate = checked_add(du, e.weight);
            if (candidate < dist[e.to]) {
                dist[e.to] = candidate;
                queue.emplace(candidate, e.to);
            }
        }
    }
    return dist;
}

// An integer-weight realization of the five fields in C-HD's MLabel.
// Field order: length, hop count, endpoint, last edge, REVERSED tail version.
struct Label {
    Distance length = INF;
    std::uint64_t hops = 0;
    Vertex vertex = 0;
    std::size_t edge = 0;
    std::uint64_t tail_version = 0;
};
inline bool less(const Label& a, const Label& b) {
    if (a.length != b.length) return a.length < b.length;
    if (a.length == INF) return false;
    if (a.hops != b.hops) return a.hops < b.hops;
    if (a.vertex != b.vertex) return a.vertex < b.vertex;
    if (a.edge != b.edge) return a.edge < b.edge;
    return a.tail_version > b.tail_version;
}
inline bool equal(const Label& a, const Label& b) {
    return !less(a, b) && !less(b, a);
}
inline Label distance_bound(Distance b) {
    if (b < 0) throw std::invalid_argument("negative bound");
    // Excludes all reachable path labels of length >= b.
    return Label{b, 0, 0, 0, 0};
}

class Labels {
public:
    explicit Labels(std::size_t n, Vertex source) : d(n), version(n, 0) {
        if (source >= n) throw std::out_of_range("invalid source");
        d[source] = Label{0, 0, source, 0, 0};
        version[source] = 1;
    }
    Label extend(Vertex u, const Edge& e) const {
        if (d.at(u).length == INF) throw std::logic_error("infinite tail");
        if (d[u].hops == std::numeric_limits<std::uint64_t>::max()) {
            throw std::overflow_error("hop counter overflow");
        }
        return Label{checked_add(d[u].length, e.weight), d[u].hops + 1,
                     e.to, e.id, version[u]};
    }
    // Equal labels are valid reconfirmations, but do not create new versions.
    bool relax(Vertex v, const Label& candidate) {
        if (less(d.at(v), candidate)) return false;
        if (less(candidate, d[v])) {
            if (version[v] == std::numeric_limits<std::uint64_t>::max()) {
                throw std::overflow_error("version counter overflow");
            }
            d[v] = candidate;
            ++version[v];
        }
        return true;
    }
    std::vector<Label> d;
    std::vector<std::uint64_t> version;
};

// Stable edge slots. Unlinking changes only the pivot-search view of the graph.
// The original Graph is not modified.
class LiveEdges {
public:
    explicit LiveEdges(const Graph& graph) : head(graph.size(), NONE) {
        if (graph.empty()) throw std::invalid_argument("empty graph");
        validate_graph(graph, 0);
        std::vector<std::size_t> all_ids;
        for (Vertex u = 0; u < graph.size(); ++u) {
            const auto& row = graph[u];
            std::vector<Vertex> targets;
            for (const auto& e : row) {
                if (e.to == u || e.id == 0) {
                    throw std::invalid_argument("local search expects no self-loops and positive edge IDs");
                }
                targets.push_back(e.to);
                all_ids.push_back(e.id);
            }
            std::sort(targets.begin(), targets.end());
            if (std::adjacent_find(targets.begin(), targets.end()) != targets.end()) {
                throw std::invalid_argument("parallel targets must be deduplicated first");
            }
            if (!std::is_sorted(row.begin(), row.end(), [](const Edge& a, const Edge& b) {
                    return std::tie(a.weight, a.to, a.id) < std::tie(b.weight, b.to, b.id);
                })) {
                throw std::invalid_argument("outgoing edges must be sorted");
            }
            if (row.empty()) continue;
            head[u] = edges.size();
            for (std::size_t j = 0; j < row.size(); ++j) {
                edges.push_back(row[j]);
                next.push_back(j + 1 < row.size() ? edges.size() : NONE);
            }
        }
        std::sort(all_ids.begin(), all_ids.end());
        if (std::adjacent_find(all_ids.begin(), all_ids.end()) != all_ids.end()) {
            throw std::invalid_argument("edge IDs must be globally unique");
        }
    }
    std::vector<Edge> edges;
    std::vector<std::size_t> head, next;
};

enum class Stop { exhausted, size_limit, contact };
struct SearchResult {
    Stop stop = Stop::exhausted;
    std::vector<Vertex> nodes;          // K, including the root and unexpanded leaves.
    std::vector<Vertex> valid;          // Root plus vertices with valid relaxation.
    std::vector<std::size_t> parents;   // First-discovery edge IDs, aligned with nodes.
    Vertex contact_from = NONE, contact_to = NONE;
    std::size_t contact_edge = NONE, contact_tree = NONE;
    std::size_t scanned = 0, deleted = 0, extracted = 0;
};

// ONE search in FindPivots-HD (FH.4--FH.22), not a complete SSSP solver.
// The caller supplies the invocation's fixed lower bound L_X, upper bound B,
// and current forest membership. It must establish the original frontier/deletion
// invariants; a numeric lower bound alone is NOT a deletion certificate.
class LocalSearch {
public:
    LocalSearch(LiveEdges& live, Labels& labels)
        : live_(live), labels_(labels), seen_(labels.d.size(), 0),
          valid_(labels.d.size(), 0), pos_(labels.d.size(), NONE),
          parent_(labels.d.size(), 0) {
        if (live.head.size() != labels.d.size()) throw std::invalid_argument("size mismatch");
    }

    SearchResult run(Vertex root, Label lower, Label upper, std::size_t limit,
                     const std::vector<std::size_t>& tree_of) {
        if (root >= seen_.size() || tree_of.size() != seen_.size() || limit < 2) {
            throw std::invalid_argument("invalid local search arguments");
        }
        if (tree_of[root] != NONE || labels_.d[root].length == INF ||
            !less(labels_.d[root], upper) || less(labels_.d[root], lower)) {
            throw std::invalid_argument("root must be unclaimed and within the bounds");
        }
        if (epoch_ == std::numeric_limits<std::uint64_t>::max()) {
            throw std::overflow_error("search stamp overflow");
        }
        ++epoch_;
        queue_.clear();
        SearchResult result;
        seen_[root] = valid_[root] = epoch_;
        parent_[root] = 0;
        pos_[root] = NONE;
        result.nodes.push_back(root);
        result.valid.push_back(root);
        activate(root);

        while (!queue_.empty() && result.nodes.size() < limit) {
            const Vertex u = extract_min();
            ++result.extracted;
            auto* link = &live_.head[u];
            while (*link != NONE) {
                const auto slot = *link;
                const auto& e = live_.edges[slot];
                const Vertex v = e.to;
                ++result.scanned;
                const Label candidate = labels_.extend(u, e);
                if (!less(candidate, upper)) break;

                if (less(labels_.d[v], lower)) {
                    *link = live_.next[slot];
                    ++result.deleted;
                    continue;
                }
                if (tree_of[v] != NONE) {
                    labels_.relax(v, candidate);
                    result.stop = Stop::contact;
                    result.contact_from = u;
                    result.contact_to = v;
                    result.contact_edge = e.id;
                    result.contact_tree = tree_of[v];
                    return finish(std::move(result));
                }

                const bool ok = labels_.relax(v, candidate);
                if (seen_[v] != epoch_) {
                    seen_[v] = epoch_;
                    valid_[v] = 0;
                    pos_[v] = NONE;
                    parent_[v] = e.id;
                    result.nodes.push_back(v);   // 松弛失败的新顶点也计入搜索名额。
                }
                if (ok) {
                    if (valid_[v] != epoch_) {
                        valid_[v] = epoch_;
                        result.valid.push_back(v);
                    }
                    activate(v);               // 先前未扩展的叶子也可能在此进入候选。
                }
                if (result.nodes.size() == limit) break;
                link = &live_.next[slot];
            }
        }
        result.stop = result.nodes.size() == limit ? Stop::size_limit : Stop::exhausted;
        return finish(std::move(result));
    }

private:
    void activate(Vertex v) {
        if (pos_[v] != NONE) return; // Keys are read from the shared label table.
        pos_[v] = queue_.size();
        queue_.push_back(v);
    }
    Vertex extract_min() {
        std::size_t best = 0;
        for (std::size_t i = 1; i < queue_.size(); ++i) {
            if (less(labels_.d[queue_[i]], labels_.d[queue_[best]])) best = i;
        }
        const Vertex u = queue_[best];
        queue_[best] = queue_.back();
        pos_[queue_[best]] = best;
        queue_.pop_back();
        pos_[u] = NONE;
        return u;
    }
    SearchResult finish(SearchResult result) {
        for (Vertex v : result.nodes) result.parents.push_back(parent_[v]);
        return result;
    }
    LiveEdges& live_;
    Labels& labels_;
    std::vector<std::uint64_t> seen_, valid_;
    std::vector<std::size_t> pos_, parent_;
    std::vector<Vertex> queue_;
    std::uint64_t epoch_ = 0;
};
} // namespace paths
