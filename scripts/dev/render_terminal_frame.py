#!/usr/bin/env python3
"""Rasterize an actual Ratatui TestBackend cell export. This is a fixture render, not a PTY screenshot."""
import argparse
import json
import re
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('frame',type=Path)
p.add_argument('output',type=Path)
a=p.parse_args()
data=json.loads(a.frame.read_text())
fonts=[ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf',15),ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf',15)]
image=Image.new('RGB',(data['width']*9,data['height']*18),(11,12,14));draw=ImageDraw.Draw(image)
def color(value,default):
    match=re.fullmatch(r'Rgb\((\d+), (\d+), (\d+)\)',value)
    return tuple(map(int,match.groups())) if match else default
for i,cell in enumerate(data['cells']):
    x=i%data['width']*9;y=i//data['width']*18
    draw.rectangle((x,y,x+8,y+17),fill=color(cell['bg'],(11,12,14)))
    draw.text((x,y-2),cell['text'],font=fonts[int(cell['bold'])],fill=color(cell['fg'],(241,243,245)))
a.output.parent.mkdir(parents=True,exist_ok=True)
image.save(a.output)
print('Saved Ratatui fixture render:',a.output)
