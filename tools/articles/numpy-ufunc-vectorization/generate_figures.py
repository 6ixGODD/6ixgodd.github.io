"""Generate the ufunc article's diagrams using the website's font stack."""
from pathlib import Path
import xml.etree.ElementTree as ET
ROOT=Path(__file__).resolve().parents[3]/'content/posts/2026-09-30-numpy-ufunc-vectorization/figures'
NS='http://www.w3.org/2000/svg';ET.register_namespace('',NS)
class Figure:
 def __init__(self,name,w,h):
  self.name=name;self.svg=ET.Element(f'{{{NS}}}svg',{'viewBox':f'0 0 {w} {h}','width':str(w),'height':str(h)})
  self.el('rect',x=0,y=0,width=w,height=h,fill='white')
 def el(self,tag,**a):return ET.SubElement(self.svg,f'{{{NS}}}{tag}',{k.replace('_','-'):str(v) for k,v in a.items()})
 def text(self,x,y,t,size=19,color='#262626',anchor='middle'):
  e=self.el('text',x=x,y=y,font_size=size,fill=color,text_anchor=anchor,font_family='"Times New Roman",Times,serif');e.text=t
 def box(self,x,y,w,h,t,accent=False):
  self.el('rect',x=x,y=y,width=w,height=h,fill='#e9eff3' if accent else '#f2f3f4',stroke='#425c70' if accent else '#aaa')
  self.text(x+w/2,y+h/2+7,t)
 def arrow(self,points,color='#425c70',dashed=False):
  self.el('polyline',points=' '.join(f'{x},{y}' for x,y in points),fill='none',stroke=color,stroke_width=1.5,stroke_dasharray='4 4' if dashed else 'none')
  x,y=points[-1];px,py=points[-2];dx,dy=x-px,y-py;length=(dx*dx+dy*dy)**.5;dx/=length;dy/=length
  self.el('polygon',points=f'{x},{y} {x-8*dx-4*dy},{y-8*dy+4*dx} {x-8*dx+4*dy},{y-8*dy-4*dx}',fill=color)
 def save(self):ET.ElementTree(self.svg).write(ROOT/self.name,encoding='utf-8',xml_declaration=True)

f=Figure('f01-broadcast-loop.svg',960,610)
f.text(480,30,'同一迭代段：长度 4 · float64 每项 8 字节',23)
x0=240;step=145
for row,y,title,values,stride in [
 (0,90,'左输入 a[0]',[0,1,2,3],'步长 8：每次读取下一个元素'),
 (1,285,'右输入 r',[10,20,30],'步长 0：本段始终读取第一个元素'),
 (2,455,'输出 result[0]',[10,11,12,13],'步长 8：每次写入下一个元素')]:
 f.text(35,y+30,title,20,anchor='start')
 for j,v in enumerate(values):
  f.box(x0+j*step,y,100,50,f'{v}.',row==1 and j==0)
  f.text(x0+j*step+75,y+72,f'{j*8} B',16,'#666')
 f.text(480,75 if row==0 else y+108,stride,18,'#666')
# Four logical reads lead to the same physical right-input element.
for j in range(4):
 x=x0+j*step+50
 f.text(x,235,f'i = {j}',18,'#425c70')
 f.arrow([(x,140),(x,211)])
 f.arrow([(x,244),(x,262),(290,262),(290,285)])
# Read/write progression is explicitly summarized rather than implying copied data.
f.text(480,435,'每步：读取左元素 + 同一个右元素 → 写入对应输出',20)
f.text(480,592,'仅示意标量循环的地址访问，不指定实际 SIMD 指令或迭代分段。',17,'#666')
f.save()

f=Figure('f02-reuse-vs-fusion.svg',1000,610)
f.text(500,30,'存储复用与循环融合',24)
for y,title,note in [(95,'保留 tmp','两次遍历 · 两个结果数组'),(265,'out=work','两次遍历 · 一个工作数组'),(435,'概念性融合循环','一次遍历 · 一个结果数组')]:
 f.text(25,y-25,title,21,anchor='start')
 f.text(975,y-25,note,18,'#666',anchor='end')
 f.box(35,y,140,55,'a')
 if y==95:
  f.box(385,y,180,55,'tmp');f.box(790,y,180,55,'affine')
  f.arrow([(175,y+27),(385,y+27)]);f.text(280,y+14,'遍历 1：× 2',18)
  f.arrow([(565,y+27),(790,y+27)]);f.text(678,y+14,'遍历 2：+ 1',18)
 elif y==265:
  f.box(385,y,180,55,'work',True)
  f.arrow([(175,y+27),(385,y+27)]);f.text(280,y+14,'遍历 1：× 2',18)
  f.arrow([(565,y+27),(745,y+27),(745,y+85),(475,y+85),(475,y+55)])
  f.text(700,y+12,'遍历 2：读取并覆盖',18)
 else:
  f.box(385,y,240,55,'寄存器中的 x × 2 + 1',True);f.box(790,y,180,55,'结果')
  f.arrow([(175,y+27),(385,y+27)]);f.text(280,y+14,'读入一个元素',18)
  f.arrow([(625,y+27),(790,y+27)]);f.text(708,y+14,'直接写出',18)
 f.el('line',x1=25,y1=y+115,x2=975,y2=y+115,stroke='#ddd')
f.text(500,578,'第三种是执行方式示意，普通 NumPy 复合表达式不自动保证循环融合。',17,'#666')
f.text(500,601,'图中为逻辑读写，不等同于 DRAM 访问次数，也不表示采用融合乘加指令。',16,'#666')
f.save()
