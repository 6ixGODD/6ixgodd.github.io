#include "shortest_paths.hpp"
#include <iostream>
#include <random>
#include <string>

using namespace paths;
void require(bool ok, const char* message) {
    if (!ok) throw std::runtime_error(message);
}
Graph make_graph(std::size_t n, const std::vector<std::tuple<Vertex, Vertex, Distance>>& raw) {
    Graph g(n);
    std::size_t id = 1;
    for (auto [u, v, w] : raw) g.at(u).push_back({v, w, id++});
    for (auto& row : g) std::sort(row.begin(), row.end(), [](const auto& a, const auto& b) {
        return std::tie(a.weight, a.to, a.id) < std::tie(b.weight, b.to, b.id);
    });
    return g;
}
const Edge& edge_to(const Graph& g, Vertex u, Vertex v) {
    for (const auto& e : g.at(u)) if (e.to == v) return e;
    throw std::runtime_error("test edge missing");
}
void seed(Labels& labels, const Graph& g, Vertex u, Vertex v) {
    auto c = labels.extend(u, edge_to(g, u, v));
    labels.relax(v, c);
}
std::vector<Distance> bellman_ford(const Graph& g, Vertex source) {
    std::vector<Distance> d(g.size(), INF);
    d[source] = 0;
    for (std::size_t step = 1; step < g.size(); ++step) {
        auto next = d;
        for (Vertex u = 0; u < g.size(); ++u) if (d[u] != INF) {
            for (const auto& e : g[u]) next[e.to] = std::min(next[e.to], checked_add(d[u], e.weight));
        }
        if (next == d) break;
        d = std::move(next);
    }
    return d;
}
void directed_cases() {
    auto g = make_graph(5, {{0,1,10},{0,2,2},{2,1,3},{1,3,1},{2,3,10}});
    require(dijkstra(g,0) == std::vector<Distance>({0,5,2,6,INF}), "stale entries and unreachable");
    auto parallel = make_graph(3, {{0,0,0},{0,1,10},{0,1,2},{1,2,0},{2,1,0}});
    require(dijkstra(parallel,0) == std::vector<Distance>({0,2,2}), "parallel edges and zero cycle");
    require(dijkstra(Graph(1),0) == std::vector<Distance>({0}), "one vertex");
    bool bad = false;
    try { dijkstra(make_graph(2, {{0,1,-1}}), 0); } catch (const std::invalid_argument&) { bad=true; }
    require(bad, "negative weights rejected");
    bad = false;
    try { dijkstra(Graph(1), 1); } catch (const std::out_of_range&) { bad=true; }
    require(bad, "bad source rejected");
    bad = false;
    try { dijkstra(make_graph(3, {{0,1,INF-1},{1,2,1}}), 0); } catch (const std::overflow_error&) { bad=true; }
    require(bad, "sentinel collision rejected");
}
void local_cases() {
    // Root x=1 has four outgoing edges. Existing labels of b,c,d,e come from r=2.
    auto g=make_graph(7, {{0,1,10},{0,2,3},{2,3,9},{2,4,10},{2,5,11},{2,6,12},
                          {1,3,20},{1,4,21},{1,5,22},{1,6,23}});
    Labels labels(g.size(),0);
    seed(labels,g,0,1); seed(labels,g,0,2);
    for (Vertex v=3;v<7;++v) seed(labels,g,2,v);
    LiveEdges live(g); LocalSearch search(live,labels);
    std::vector<std::size_t> forest(g.size(),NONE);
    auto r=search.run(1,labels.d[1],distance_bound(40),4,forest);
    require(r.stop==Stop::size_limit, "counted failed leaves stop at limit");
    require(r.nodes==std::vector<Vertex>({1,3,4,5}), "root included in count");
    require(r.valid==std::vector<Vertex>({1}), "failed leaves not activated");
    require(r.scanned==3 && r.extracted==1, "fourth edge not scanned");
    require(labels.d[3].length==12 && labels.d[5].length==14, "failed labels unchanged");
    std::cout << "F04: nodes=" << r.nodes.size() << " valid=" << r.valid.size()
              << " scanned=" << r.scanned << " extracted=" << r.extracted << " stop=size_limit\n";

    // Leaf first seen through x, then promoted through y. Its discovery parent stays x.
    auto p=make_graph(4, {{0,1,10},{0,2,15},{1,2,10},{1,3,1},{3,2,2}});
    Labels lp(p.size(),0); seed(lp,p,0,1); seed(lp,p,0,2);
    LiveEdges vp(p); LocalSearch sp(vp,lp); std::vector<std::size_t> fp(p.size(),NONE);
    auto rp=sp.run(1,lp.d[1],Label{},8,fp);
    require(rp.stop==Stop::exhausted && lp.d[2].length==13, "leaf promotion improves label");
    auto where=std::find(rp.nodes.begin(),rp.nodes.end(),Vertex{2})-rp.nodes.begin();
    require(rp.parents[static_cast<std::size_t>(where)]==edge_to(p,1,2).id, "discovery parent not rewritten");
    require(lp.d[2].edge==edge_to(p,3,2).id, "distance predecessor changes independently");

    // Safe deletion: true distance of x is 10, target has distance 3 (< L=10).
    auto q=make_graph(3, {{0,1,10},{0,2,3},{1,2,1}});
    Labels lq(q.size(),0); seed(lq,q,0,1); seed(lq,q,0,2);
    LiveEdges vq(q); LocalSearch sq(vq,lq); std::vector<std::size_t> fq(q.size(),NONE);
    auto rq=sq.run(1,lq.d[1],Label{},4,fq);
    require(rq.deleted==1 && vq.head[1]==NONE && q[1].size()==1, "live-only deletion");
    auto rq2=sq.run(1,lq.d[1],Label{},4,fq);
    require(rq2.scanned==0, "deleted edge never rescanned");

    // Contact stops before absorbing the existing tree's vertex in K.
    auto c=make_graph(3, {{0,1,10},{1,2,1}});
    Labels lc(c.size(),0); seed(lc,c,0,1);
    LiveEdges vc(c); LocalSearch sc(vc,lc); std::vector<std::size_t> fc(c.size(),NONE); fc[2]=7;
    auto rc=sc.run(1,lc.d[1],Label{},5,fc);
    require(rc.stop==Stop::contact && rc.contact_tree==7 && rc.nodes.size()==1, "contact event");
    require(lc.d[2].length==11, "contact still relaxes target");

    // Upper bound is exclusive, even when edge has zero weight.
    auto b=make_graph(3, {{0,1,2},{0,2,3}});
    Labels lb(b.size(),0); LiveEdges vb(b); LocalSearch sb(vb,lb);
    std::vector<std::size_t> fb(b.size(),NONE);
    auto rb=sb.run(0,lb.d[0],distance_bound(3),10,fb);
    require(lb.d[1].length==2 && lb.d[2].length==INF, "exclusive upper bound");
    require(rb.stop==Stop::exhausted, "bounded exhaustion");

    Label older{5,2,3,7,1}, newer{5,2,3,7,2};
    require(less(newer,older) && !less(older,newer), "tail version order is reversed");
    auto saved=lc.version[2];
    require(lc.relax(2,lc.d[2]) && lc.version[2]==saved, "equality reconfirms without version increment");
}
void random_cases() {
    std::mt19937_64 rng(20260930);
    for (int trial=0;trial<2500;++trial) {
        const std::size_t n=1+rng()%20;
        std::vector<std::tuple<Vertex,Vertex,Distance>> raw;
        for (Vertex u=0;u<n;++u) for (Vertex v=0;v<n;++v) {
            if (u!=v && rng()%100<23) raw.emplace_back(u,v,static_cast<Distance>(rng()%15));
        }
        auto g=make_graph(n,raw); const Vertex s=rng()%n;
        auto want=bellman_ford(g,s);
        require(dijkstra(g,s)==want, "Dijkstra/Bellman-Ford differential");
        Labels l(n,s); LiveEdges live(g); LocalSearch search(live,l);
        std::vector<std::size_t> forest(n,NONE);
        auto r=search.run(s,l.d[s],Label{},n+1,forest);
        require(r.stop==Stop::exhausted, "uncapped single search must exhaust");
        for(Vertex v=0;v<n;++v) require(l.d[v].length==want[v], "local closure/Bellman-Ford differential");
        // New state for a genuinely truncated search: never claim partial labels are final.
        Labels lc(n,s); LiveEdges vc(g); LocalSearch sc(vc,lc);
        const std::size_t cap=2+rng()%6;
        auto rc=sc.run(s,lc.d[s],Label{},cap,forest);
        require(rc.nodes.size()<=cap, "size invariant");
        auto nodes=rc.nodes; std::sort(nodes.begin(),nodes.end());
        require(std::adjacent_find(nodes.begin(),nodes.end())==nodes.end(), "no duplicate discovered nodes");
        for(Vertex v=0;v<n;++v) require(lc.d[v].length>=want[v], "labels remain path upper bounds");
    }
}
int main() {
    directed_cases(); local_cases(); random_cases();
    std::cout << "PASS: directed cases, targeted local-search scenarios, 2500 seeded random graphs\n";
    std::cout << "Dijkstra and untruncated local search both match independent Bellman-Ford.\n";
    std::cout << "This is NOT a verification of the complete C-HD algorithm.\n";
}
