from PIL import Image
import sys
import json

pixels = []

img = Image.open(sys.argv[1]).convert('RGBA')

_,_,w,h = img.getbbox()

for x in range(w):
    for y in range(h):
        r,g,b,a = img.getpixel((x,y))
        pixels.append({'x': x, 'y': y, 'p': ('%02x%02x%02x%02x' % (r,g,b,a))})

with open('img.json', 'w') as f:
    f.write(json.dumps(pixels))
