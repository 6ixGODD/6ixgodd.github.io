# /// script
# requires-python = ">=3.10"
# dependencies = ["svgwrite==1.4.3"]
# ///
"""Regenerate the article's eight diagrams: uv run tools/articles/dijkstra-vs-chd/generate_figures.py.

SVG text deliberately uses the website's exact serif font stack. Chinese glyphs
therefore use the same browser font fallback as article text; no outlined fonts.
"""
from pathlib import Path
import math
import svgwrite

ROOT = Path(__file__).resolve().parents[3] / "content/posts/2026-09-30-dijkstra-vs-chd/figures"
INK, MUTED, LIGHT, ACCENT = '#262626', '#666666', '#f2f3f4', '#425c70'
FONT = '"Times New Roman",Times,serif'

class Figure:
    def __init__(self, name, width, height):
        self.d = svgwrite.Drawing(str(ROOT / name), size=(width, height), viewBox=f'0 0 {width} {height}')
        self.d.add(self.d.rect((0, 0), (width, height), fill='white'))
        self.d.defs.add(self.d.style(f'text{{font-family:{FONT}}}'))
    def text(self, x, y, text, size=18, anchor='start', color=INK, weight='normal'):
        self.d.add(self.d.text(text, insert=(x, y), font_size=size, text_anchor=anchor, fill=color, font_weight=weight))
    def box(self, x, y, w, h, lines=(), fill='white', dashed=False, size=18):
        opts = dict(fill=fill, stroke='#999', stroke_width=1)
        if dashed: opts['stroke_dasharray'] = '5 4'
        self.d.add(self.d.rect((x,y),(w,h),**opts))
        for i, line in enumerate(lines):
            self.text(x+w/2, y+h/2+(i-(len(lines)-1)/2)*25+6, line, size, 'middle')
    def line(self, points, color=INK, dashed=False, arrow=False, width=1.5):
        opts = dict(fill='none', stroke=color, stroke_width=width)
        if dashed: opts['stroke_dasharray'] = '5 4'
        self.d.add(self.d.polyline(points, **opts))
        if arrow:
            (ax, ay), (bx, by) = points[-2:]
            dx, dy = bx-ax, by-ay
            length = math.hypot(dx, dy)
            dx, dy = dx/length, dy/length
            self.d.add(self.d.polygon([(bx, by), (bx-8*dx+4*dy, by-8*dy-4*dx),
                                       (bx-8*dx-4*dy, by-8*dy+4*dx)], fill=color))
    def edge(self, a, b, label='', dashed=False, color=INK, label_offset=(0,-8), radius=23):
        dx,dy=b[0]-a[0],b[1]-a[1]; length=math.hypot(dx,dy)
        start=(a[0]+radius*dx/length,a[1]+radius*dy/length)
        end=(b[0]-(radius+3)*dx/length,b[1]-(radius+3)*dy/length)
        self.line([start,end],color,dashed,True)
        if label:
            x,y=(a[0]+b[0])/2+label_offset[0],(a[1]+b[1])/2+label_offset[1]
            self.d.add(self.d.rect((x-15,y-17),(30,23),fill='white'))
            self.text(x,y,label,17,'middle',color)
    def node(self,x,y,label,state='open',distance=None,size=18):
        self.d.add(self.d.circle((x,y),23,fill='#e4e7e9' if state!='open' else 'white',stroke=INK,stroke_width=1.5))
        if state=='current': self.d.add(self.d.circle((x,y),19,fill='none',stroke=INK,stroke_width=1))
        self.text(x,y+6,label,size,'middle')
        if distance is not None: self.text(x,y+45,f'd={distance}',16,'middle',MUTED)
    def save(self): self.d.save(pretty=True)

# F01: repeated layout, distance labels separated from edge weights.
f=Figure('f01-dijkstra.svg',1020,405)
for panel,(current,done,dist) in enumerate([
    ('s',{'s'}, {'s':0,'a':10,'b':2,'t':'∞','z':'∞'}),
    ('b',{'s','b'}, {'s':0,'a':5,'b':2,'t':12,'z':'∞'}),
    ('a',{'s','b','a'}, {'s':0,'a':5,'b':2,'t':6,'z':'∞'}),
]):
    off=panel*340
    f.text(off+170,28,f'处理 {current} 后',20,'middle',weight='bold')
    points={'s':(off+48,170),'a':(off+155,90),'b':(off+155,250),'t':(off+284,170),'z':(off+282,292)}
    for u,v,w,delta in [('s','a','10',(0,-7)),('s','b','2',(-7,12)),('b','a','3',(18,0)),('b','t','10',(5,12)),('a','t','1',(4,-8))]:
        highlight=(current=='s' and u=='s') or (current=='b' and u=='b') or (current=='a' and u=='a')
        f.edge(points[u],points[v],w,color=ACCENT if highlight else MUTED,label_offset=delta)
    for label,(x,y) in points.items():
        state='current' if label==current else 'settled' if label in done else 'open'
        f.node(x,y,label,state,None if label=='a' else dist[label])
        if label=='a': f.text(x,y-34,f'd={dist[label]}',16,'middle',MUTED)
    if panel<2: f.line([(off+336,12),(off+336,344)],'#ddd',width=1)
f.node(45,375,'','settled');f.text(78,381,'已确定',16)
f.node(230,375,'','current');f.text(263,381,'本阶段处理',16)
f.node(455,375,'');f.text(488,381,'尚未确定；距离可继续改善',16)
f.save()

# F02: only the prefix is asserted to be in S; no state claim for the suffix.
f=Figure('f02-greedy-boundary.svg',1000,370)
f.box(25,65,360,145,fill=LIGHT)
f.text(50,91,'已确定集合 S',18,weight='bold')
for a,b in [((100,143),(300,143)),((300,143),(525,143))]:f.edge(a,b,color=ACCENT if a[0]==300 else INK)
f.line([(550,143),(625,143),(690,120),(745,143),(850,143)],dashed=True,arrow=True)
for x,label,state in [(100,'s','settled'),(300,'x','settled'),(525,'y','open'),(875,'u','open')]:f.node(x,143,label,state)
f.text(300,193,'x ∈ S',17,'middle');f.text(525,193,'y ∉ S',17,'middle');f.text(875,193,'u ∉ S',17,'middle')
f.text(700,97,'剩余最短路径；中间顶点状态未限定',16,'middle',MUTED)
f.text(500,246,'δ(u) ≤ d[u] ≤ d[y] ≤ δ(y) ≤ δ(u)',24,'middle')
f.box(25,275,300,60,['本轮最小值选择','d[u] ≤ d[y]'],size=17)
f.box(350,275,300,60,['边 x → y 已松弛','d[y] ≤ δ(y)'],size=17)
f.box(675,275,300,60,['剩余路径边权非负','δ(y) ≤ δ(u)'],size=17)
f.text(30,361,'一般情形示意；允许 y = u。',16,color=MUTED)
f.save()

# F03: half-open intervals, request upper bound distinguished from actual return.
f=Figure('f03-recursive-intervals.svg',1000,335)
f.text(30,27,'横轴：规范最短路标签的次序',19,weight='bold')
f.text(30,82,'父调用',17)
f.box(140,52,650,50,['实际处理 [A, D)'],fill=LIGHT)
f.box(790,52,155,50,['未处理尾部'],dashed=True,size=16)
f.text(140,128,'A',18,'middle');f.text(790,128,"D = B′ₓ",18,'middle');f.text(945,128,'请求上界 Bₓ',16,'middle')
f.text(30,194,'子调用',17)
f.box(220,163,210,50,['[a₁, b₁)'],fill=LIGHT)
f.box(555,163,150,50,['[a₂, b₂)'],fill=LIGHT)
for x,label in [(220,'a₁'),(430,'b₁'),(555,'a₂'),(705,'b₂')]:
    f.line([(x,104),(x,155)],'#bbb',True,width=1)
    f.text(x,238,label,18,'middle')
f.line([(130,266),(955,266)],arrow=True)
f.text(545,304,'A ≤ a₁ < b₁ ≤ a₂ < b₂ ≤ D；空白不要求由子调用覆盖',18,'middle')
f.save()

# F04: no fictitious activation on a failed relaxation.
f=Figure('f04-failed-relaxations.svg',1000,460)
f.text(30,29,'下界 L = 10    上界 B = 40    容量 k = 4',19,weight='bold')
root=(170,210)
for i,y in enumerate([80,170,260,350],1):
    target=(630,y);f.edge(root,target,str(19+i),dashed=i==4,color=MUTED,label_offset=(-25,-8))
    f.node(*target,f'v{i}')
    if i<=3:
        f.text(692,y-7,f'旧标签 {11+i}；候选 {29+i}',18)
        f.text(692,y+19,'松弛失败；计入 K，未进入 H',16,color=MUTED)
    else:
        f.text(692,y-7,'旧标签 15',18)
        f.text(692,y+19,'尚未扫描；候选 33 未计算',16,color=MUTED)
f.node(*root,'x','current',10)
f.box(30,396,940,45,['停止时：K = {x, v1, v2, v3}    有效集合 = {x}    H = ∅'],fill=LIGHT,size=18)
f.save()

# F05: interval ordering, deliberately unsorted values inside blocks.
f=Figure('f05-block-merge.svg',1020,320)
intervals=['[0, 10)','[10, 20)','[20, 30)','[30, 40)']
values=['7, 2, 9','18, 11, 16','28, 21, 25','37, 31, 35']
f.text(250,27,'子结构',19,'middle',weight='bold');f.text(760,27,'父结构',19,'middle',weight='bold')
for i in range(4):
    x=35+i*250
    f.box(x,47,205,75,[intervals[i],values[i]],fill=LIGHT,size=18)
    if i in [0,2]:f.line([(x+205,85),(x+244,85)],arrow=True)
f.text(510,155,'合并：复用满足区间次序的块',18,'middle')
for i in range(4):
    x=35+i*250;f.box(x,179,205,75,[intervals[i],values[i]],fill=LIGHT,size=18)
    if i<3:f.line([(x+205,217),(x+244,217)],color=ACCENT if i==1 else INK,arrow=True,width=2 if i==1 else 1.5)
f.text(510,279,'新增块链接',16,'middle',ACCENT)
f.text(510,308,'块内没有全序；分裂、清理与候选提取仍须单独计费。',17,'middle',MUTED)
f.save()

# F06: proof hierarchy, no edge from unverified C++ to Lean theorems.
f=Figure('f06-proof-dependencies.svg',1020,720)
f.box(350,15,320,60,['图与规范标签定义'],fill=LIGHT)
for x,lines in [(30,['局部搜索契约']),(365,['永久删边引理']),(700,['分组和分块队列契约'])]:
    f.box(x,120,290,60,lines)
    f.line([(510,75),(510,95),(x+145,95),(x+145,115)],arrow=True)
f.box(180,240,300,65,['递归正确性'],fill=LIGHT)
f.box(600,240,360,65,['全局成本', '计数、摊还界与参数选择'],fill=LIGHT,size=17)
for x in [175,510,845]:f.line([(x,180),(x,208),(330,208),(330,235)],arrow=True)
f.line([(845,180),(845,235)],arrow=True)
f.box(30,355,280,80,['具体 RAM 程序','与抽象算法的一致性'],size=17)
f.box(350,430,350,65,['chd_exact_within'],fill=LIGHT,size=21)
f.line([(330,305),(330,386),(410,386),(410,425)],arrow=True)
f.line([(780,305),(780,388),(635,388),(635,425)],arrow=True)
f.line([(310,395),(330,395),(330,462),(345,462)],arrow=True)
f.box(735,442,260,100,['本文 C++：仅测试','未建立形式化细化证明'],dashed=True,size=17)
f.box(350,545,350,55,['chd_CHDTarget'],size=21)
f.line([(525,495),(525,540)],arrow=True)
f.box(350,643,350,55,['chd_gateC'],size=21)
f.line([(525,600),(525,638)],arrow=True)
f.text(760,620,'新颖性评审与实际性能',16,color=MUTED)
f.text(760,645,'不在此图的定理结论中。',16,color=MUTED)
f.text(32,688,'简化证明层次图，',16,color=MUTED);f.text(32,711,'不是运行流程或模块导入图。',16,color=MUTED)
f.save()

# F07: all original incoming edges go to u0; zero chain edges are directed.
f=Figure('f07-degree-splitting.svg',1020,490)
f.text(230,27,'拆分前',20,'middle',weight='bold');f.text(770,27,'拆分后（Δ = 3）',20,'middle',weight='bold')
ys=[90,170,250,330,410];weights=[1,4,2,5,3]
f.line([(20,250),(134,250)],arrow=True);f.text(78,225,'原图入边',16,'middle')
for i,(y,w) in enumerate(zip(ys,weights),1):
    f.edge((160,250),(405,y),str(w),label_offset=(-12,-8))
    f.node(405,y,f'v{i}')
f.node(160,250,'u');f.text(160,301,'出度 5',17,'middle')
f.line([(490,250),(530,250)],arrow=True)
roots=[(670,115),(670,250),(670,385)]
f.line([(540,115),(642,115)],arrow=True);f.text(582,88,'原图入边',16,'middle')
for i,xy in enumerate(roots):
    if i<2:f.edge(xy,roots[i+1],'0',color=ACCENT,label_offset=(-20,0))
for i,(y,w) in enumerate(zip(ys,weights),1):
    parent=roots[(i-1)//2]
    f.edge(parent,(940,y),str(w),label_offset=(12,-8));f.node(940,y,f'v{i}')
for i,xy in enumerate(roots):
    f.node(*xy,f'u{i}');f.text(xy[0]-52,xy[1]+41,f'出度 {3 if i<2 else 1}',16,'end')
f.text(755,464,'u 由 u0 表示；δ(u0) = δ(u1) = δ(u2)',18,'middle')
f.save()

# F08: numerical curves of symbolic expressions, NOT benchmark timings.
f=Figure('f08-parameter-balance.svg',960,515)
f.text(45,28,'参数平衡：t₀ = (A/m)²ᐟ³',21,weight='bold')
f.text(45,55,'纵轴：成本除以 m t₀（无量纲）',17,color=MUTED)
x0,x1,y0,y1=105,890,80,410
X=lambda q:x0+(math.log2(q)+3)/6*(x1-x0)
Y=lambda val:y1-val/8.5*(y1-y0)
for val in [0,1,2,4,6,8]:
    f.line([(x0,Y(val)),(x1,Y(val))],'#e4e4e4',width=1);f.text(x0-15,Y(val)+6,str(val),17,'end')
for q,label in [(1/8,'1/8'),(1/4,'1/4'),(1/2,'1/2'),(1,'1'),(2,'2'),(4,'4'),(8,'8')]:
    f.line([(X(q),y0),(X(q),y1)],'#eeeeee',width=1);f.text(X(q),y1+27,label,17,'middle')
f.line([(x0,y0),(x0,y1),(x1+10,y1)],width=1.5)
qs=[2**(-3+6*i/300) for i in range(301)]
f.line([(X(q),Y(q**-.5)) for q in qs],ACCENT,width=2.5)
f.line([(X(q),Y(q)) for q in qs],INK,dashed=True,width=2)
f.text(168,173,'A/√t → q⁻¹ᐟ²',19,color=ACCENT)
f.text(760,118,'mt → q',19)
f.d.add(f.d.circle((X(1),Y(1)),5,fill=INK))
f.line([(X(1),Y(1)-7),(X(1)+70,Y(1)-65)],MUTED,width=1)
f.text(X(1)+76,Y(1)-66,'(1, 1)：两项同阶',18)
f.text((x0+x1)/2,470,'q = t / t₀（对数刻度）',19,'middle')
f.text((x0+x1)/2,503,'理论表达式；交点并非两项之和的精确最优点。',17,'middle',MUTED)
f.save()
print('Generated eight SVG figures in', ROOT)

# The two browser fallback images are served as PNG; preserve their vector sources here.
import shutil
import subprocess
for name in ("f05-block-merge", "f06-proof-dependencies"):
    svg = ROOT / f"{name}.svg"
    subprocess.run(["sips", "-s", "format", "png", str(svg), "--out", str(ROOT / f"{name}.png")], check=True)
    shutil.move(str(svg), str(Path(__file__).resolve().parent / "figure-sources" / svg.name))
