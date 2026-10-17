"""Original mathematical test skateboard; no input game/model/image files.

Outputs CodeWalker XML and own solid-color DDS. Run the companion PowerShell
builder to serialize and verify the FiveM resource. Units are metres.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import xml.etree.ElementTree as ET

NAME = 'gtaxskate_test_board'
COLORS = [('grip', (32, 34, 39)), ('deck', (224, 52, 112)),
          ('edge', (190, 143, 90)), ('metal', (146, 155, 166)),
          ('wheel', (236, 224, 189))]


def tag(parent, element, text=None, **attrs):
    node = ET.SubElement(parent, element, {k: str(v) for k, v in attrs.items()})
    node.text = text
    return node


def vector(parent, name, values):
    return tag(parent, name, **dict(zip('xyz', (f'{v:.7f}' for v in values))))


def save_xml(path, root):
    ET.indent(root)
    ET.ElementTree(root).write(path, encoding='utf-8', xml_declaration=True)


def generate(destination):
    destination.mkdir(parents=True, exist_ok=True)
    parts = [[] for _ in COLORS]

    def tri(material, a, b, c):
        u, v = [b[i]-a[i] for i in range(3)], [c[i]-a[i] for i in range(3)]
        n = (u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0])
        length = math.sqrt(sum(x*x for x in n))
        if length < 1e-10:
            raise ValueError('degenerate generated triangle')
        parts[material].append((a, b, c, tuple(x/length for x in n)))

    def box(material, lo, hi):
        v = [(x, y, z) for z in (lo[2], hi[2]) for y in (lo[1], hi[1]) for x in (lo[0], hi[0])]
        for face in ((0,2,3,1), (4,5,7,6), (0,1,5,4), (2,6,7,3), (0,4,6,2), (1,3,7,5)):
            a,b,c,d = [v[i] for i in face]
            tri(material,a,b,c); tri(material,a,c,d)

    outline = []
    for center, start in ((.295, -90), (-.295, 90)):
        for i in range(17):
            angle = math.radians(start + i*180/16)
            x, y = center + .105*math.cos(angle), .105*math.sin(angle)
            z = .025 * max(0, (abs(x)-.27)/.13)**2
            outline.append((x,y,z))
    for i, a in enumerate(outline):
        b = outline[(i+1) % len(outline)]
        aa, bb = (a[0],a[1],a[2]-.012), (b[0],b[1],b[2]-.012)
        tri(0,(0,0,0),a,b)
        tri(1,(0,0,-.012),bb,aa)
        tri(2,a,aa,bb); tri(2,a,bb,b)

    def cylinder(material, x, y, z, radius, depth):
        for i in range(16):
            a, b = 2*math.pi*i/16, 2*math.pi*(i+1)/16
            p = (x+radius*math.cos(a), y-depth/2, z+radius*math.sin(a))
            q = (x+radius*math.cos(b), y-depth/2, z+radius*math.sin(b))
            r = (q[0],y+depth/2,q[2]); s = (p[0],y+depth/2,p[2])
            tri(material,p,s,r); tri(material,p,r,q)
            tri(material,(x,y-depth/2,z),p,q)
            tri(material,(x,y+depth/2,z),r,s)

    for x in (-.24,.24):
        box(3,(x-.032,-.035,-.025),(x+.032,.035,-.012))
        box(3,(x-.014,-.015,-.05),(x+.014,.015,-.025))
        cylinder(3,x,0,-.052,.008,.19)
        for y in (-.095,.095):
            cylinder(4,x,y,-.052,.028,.025)

    vertices = [v for part in parts for triangle in part for v in triangle[:3]]
    lo = tuple(min(v[i] for v in vertices) for i in range(3))
    hi = tuple(max(v[i] for v in vertices) for i in range(3))
    center = tuple((lo[i]+hi[i])/2 for i in range(3))
    radius = max(math.dist(v,center) for v in vertices)
    drawable = ET.Element('Drawable')
    tag(drawable,'Name',NAME)
    vector(drawable,'BoundingSphereCenter',center); tag(drawable,'BoundingSphereRadius',value=radius)
    vector(drawable,'BoundingBoxMin',lo); vector(drawable,'BoundingBoxMax',hi)
    for suffix in ('High','Med','Low','Vlow'):
        tag(drawable,'LodDist'+suffix,value=120)
        tag(drawable,'Flags'+suffix,value=1 if suffix=='High' else 0)
    shaders = tag(tag(drawable,'ShaderGroup'),'Shaders')
    for color,_ in COLORS:
        shader=tag(shaders,'Item'); tag(shader,'Name','default'); tag(shader,'FileName','default.sps')
        tag(shader,'RenderBucket',value=0)
        sampler=tag(tag(shader,'Parameters'),'Item',name='DiffuseSampler',type='Texture')
        tag(sampler,'Name',NAME+'_'+color)
    model=tag(tag(drawable,'DrawableModelsHigh'),'Item')
    for key,value in [('RenderMask',255),('Flags',0),('HasSkin',0),('BoneIndex',0),('Unknown1',0)]:
        tag(model,key,value=value)
    geometries=tag(model,'Geometries')
    for index,part in enumerate(parts):
        geometry=tag(geometries,'Item'); tag(geometry,'ShaderIndex',value=index)
        vector(geometry,'BoundingBoxMin',lo); vector(geometry,'BoundingBoxMax',hi)
        buffer=tag(geometry,'VertexBuffer'); tag(buffer,'Flags',value=0)
        layout=tag(buffer,'Layout',type='GTAV1')
        for name in ('Position','Normal','Colour0','TexCoord0'): tag(layout,name)
        lines=[]
        for triangle in part:
            for vertex in triangle[:3]:
                lines.append(' '.join(f'{x:.7f}' for x in (*vertex,*triangle[3]))+' 255 255 255 255 0.5 0.5')
        tag(buffer,'Data','\n'+'\n'.join(lines)+'\n')
        tag(tag(geometry,'IndexBuffer'),'Data',' '.join(str(i) for i in range(len(lines))))
    # Own conservative convex box collision; presentation mesh remains detailed.
    bounds=tag(drawable,'Bounds',type='Composite')
    def bound_common(node, low, high):
        mid=tuple((low[i]+high[i])/2 for i in range(3))
        for name,values in [('BoxMin',low),('BoxMax',high),('BoxCenter',mid),('SphereCenter',mid)]: vector(node,name,values)
        tag(node,'SphereRadius',value=radius); tag(node,'Margin',value=.002)
        tag(node,'Volume',value=math.prod(high[i]-low[i] for i in range(3)))
        vector(node,'Inertia',(.02,.08,.08))
        for name,value in [('MaterialIndex',0),('MaterialColourIndex',0),('ProceduralID',0),('RoomID',0),('PedDensity',0),('UnkFlags',0),('PolyFlags',0),('UnkType',1)]: tag(node,name,value=value)
    bound_common(bounds,lo,hi)
    child=tag(tag(bounds,'Children'),'Item',type='Box'); bound_common(child,lo,hi)
    tag(child,'CompositeTransform','1 0 0 0\n0 1 0 0\n0 0 1 0\n0 0 0 1')
    tag(child,'CompositeFlags1','MAP_DYNAMIC, MAP_VEHICLE')
    tag(child,'CompositeFlags2','VEHICLE_NOT_BVH, VEHICLE_BVH, PED, RAGDOLL, OBJECT, TEST_SCRIPT')
    tag(drawable,'Lights')
    save_xml(destination/(NAME+'.ydr.xml'),drawable)

    textures=ET.Element('TextureDictionary')
    assets=destination/NAME; assets.mkdir(exist_ok=True)
    for color,rgb in COLORS:
        item=tag(textures,'Item'); texture=NAME+'_'+color
        tag(item,'Name',texture)
        for name,value in [('Unk32',128),('ExtraFlags',0),('Width',4),('Height',4),('MipLevels',1)]: tag(item,name,value=value)
        tag(item,'Usage','DIFFUSE'); tag(item,'UsageFlags','UNK24')
        tag(item,'Format','D3DFMT_A8R8G8B8'); tag(item,'FileName',texture+'.dds')
        header=[124,0x100F,4,4,16,0,1]+[0]*11+[32,0x41,0,32,0x00FF0000,0x0000FF00,0x000000FF,0xFF000000,0x1000,0,0,0,0]
        assert len(header)==31
        (assets/(texture+'.dds')).write_bytes(b'DDS '+struct.pack('<31I',*header)+bytes((rgb[2],rgb[1],rgb[0],255))*16)
    save_xml(destination/(NAME+'.ytd.xml'),textures)

    ytyp=ET.Element('CMapTypes'); tag(ytyp,'extensions')
    archetype=tag(tag(ytyp,'archetypes'),'Item',type='CBaseArchetypeDef')
    for key,value in [('lodDist',120),('flags',32),('specialAttribute',0)]:tag(archetype,key,value=value)
    vector(archetype,'bbMin',lo); vector(archetype,'bbMax',hi); vector(archetype,'bsCentre',center)
    tag(archetype,'bsRadius',value=radius); tag(archetype,'hdTextureDist',value=15)
    for key,value in [('name',NAME),('textureDictionary',NAME),('clipDictionary',None),('drawableDictionary',None),('physicsDictionary',None),('assetType','ASSET_TYPE_DRAWABLE'),('assetName',NAME),('extensions',None)]:tag(archetype,key,value)
    tag(ytyp,'name','gtaxskate_board'); tag(ytyp,'dependencies'); tag(ytyp,'compositeEntityTypes')
    save_xml(destination/'gtaxskate_board.ytyp.xml',ytyp)
    receipt={'schema':1,'model':NAME,'provenance':'Original mathematical geometry and solid RGB colors; no game input assets',
             'generatorSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'triangles':sum(map(len,parts)),'vertices':len(vertices),'boundsMin':lo,'boundsMax':hi,
             'longAxis':'X','deckStandingTopZ':0,'wheelContactZ':-.08,'units':'metres'}
    (destination/'provenance.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(receipt))


if __name__=='__main__':
    parser=argparse.ArgumentParser(); parser.add_argument('output',type=Path)
    generate(parser.parse_args().output)
