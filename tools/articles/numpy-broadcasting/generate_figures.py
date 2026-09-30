"""Regenerate the broadcasting diagram with the site's serif font stack."""
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
SVG = 'http://www.w3.org/2000/svg'
ET.register_namespace('', SVG)
svg = ET.Element(f'{{{SVG}}}svg', {'viewBox': '0 0 940 570', 'width': '940', 'height': '570'})
def el(tag, **attrs):
    return ET.SubElement(svg, f'{{{SVG}}}{tag}', {k.replace('_', '-'): str(v) for k,v in attrs.items()})
def text(x,y,value,size=19,color='#262626',anchor='middle'):
    t=el('text',x=x,y=y,font_size=size,fill=color,text_anchor=anchor,font_family='"Times New Roman",Times,serif');t.text=value

def box(x,y,w,h,value,selected=False,reused=False):
    el('rect',x=x,y=y,width=w,height=h,fill='#e9eff3' if selected else '#f2f3f4' if reused else 'white',stroke='#425c70' if selected else '#aaa',stroke_width=2 if selected else 1)
    text(x+w/2,y+h/2+7,value)
def arrow(points):
    el('polyline',points=' '.join(f'{x},{y}' for x,y in points),fill='none',stroke='#425c70',stroke_width=1.6)
    x,y=points[-1];el('polygon',points=f'{x},{y} {x-5},{y-9} {x+5},{y-9}',fill='#425c70')
el('rect',x=0,y=0,width=940,height=570,fill='white')
for side,x in [('r',80),('b',540)]:
    center=x+160
    text(center,30,'rr：沿列复用' if side=='r' else 'bb：沿行复用',23)
    text(center,60,'形状 (3, 4) · 步长 (4, 0)' if side=='r' else '形状 (3, 4) · 步长 (0, 4)',18)
    text(center,86,'逻辑视图 · 非独立缓冲区',16,'#666')
    for j in range(4):text(x+40+80*j,116,str(j),16,'#666')
    for i in range(3):
        text(x-20,150+50*i,str(i),16,'#666')
        for j in range(4):
            box(x+80*j,128+50*i,80,50,str(i if side=='r' else 10+10*j),i==2 and j==1,(i==2 if side=='r' else j==1))
    text(center+60,309,'共同索引 (2, 1)',18,'#425c70')
    text(center,485,'r 的实际存储：3 个 int32' if side=='r' else 'b 的实际存储：4 个 int32',18)
    sx=x+40 if side=='r' else x
    for j in range(3 if side=='r' else 4):
        box(sx+80*j,389,80,48,str(j if side=='r' else 10+10*j),j==(2 if side=='r' else 1))
        text(sx+40+80*j,460,str(4*j),16,'#666')
    text(x-20,460,'偏移',15,'#666',anchor='end')
arrow([(200,278),(200,332),(320,332),(320,389)])
text(120,354,'读取偏移 8 字节',16,'#425c70')
arrow([(660,278),(660,389)])
text(684,340,'读取偏移 4 字节',16,'#425c70',anchor='start')
text(470,520,'结果 (2, 1)：2 + 20 = 22',23)
text(470,548,'零步长复用已有元素；每个视图只引用自己的输入数据。',17,'#666')
path=ROOT/'content/posts/2026-09-30-numpy-broadcasting/figures/f01-broadcast-addresses.svg'
ET.ElementTree(svg).write(path,encoding='utf-8',xml_declaration=True)
