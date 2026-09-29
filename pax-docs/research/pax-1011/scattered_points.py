"""PAX-1011 numerical design probe. CPU reference, not Pax implementation.

Requires NumPy and Pillow. Run with the bundled Python runtime.
Natural-neighbor weights use direct Voronoi half-plane clipping, not triangle
color interpolation. Sibson C1 uses the documented formula with zero gradients.
"""
from pathlib import Path
import argparse
import json
import time
import numpy as np
from PIL import Image, ImageDraw, ImageFont

POINTS = np.array([[0,0],[1,0],[1,1],[0,1],[.27,.32],[.71,.37],[.48,.78]], dtype=float)
COLORS = np.array([[0,1,1,1],[1,0,1,1],[1,1,0,1],[1,0,1,1],[1,1,0,1],[0,1,1,1],[1,0,1,1]], dtype=float)

def clip(poly, nx, ny, bound):
    if not poly:
        return []
    out=[]
    prev=poly[-1]
    dp=nx*prev[0]+ny*prev[1]-bound
    for cur in poly:
        dc=nx*cur[0]+ny*cur[1]-bound
        if (dc <= 0) != (dp <= 0):
            t=dp/(dp-dc)
            out.append((prev[0]+t*(cur[0]-prev[0]), prev[1]+t*(cur[1]-prev[1])))
        if dc <= 0:
            out.append(cur)
        prev, dp=cur, dc
    return out

def area(poly):
    if len(poly)<3:
        return 0.
    # Translate before summing to avoid cancellation for remote Voronoi vertices.
    ox,oy=poly[0]
    return abs(sum((poly[i][0]-ox)*(poly[i+1][1]-oy)-(poly[i+1][0]-ox)*(poly[i][1]-oy)
                   for i in range(1,len(poly)-1)))*.5

def hull(points):
    def cross(a,b,c):
        return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
    ids=sorted(range(len(points)),key=lambda i:tuple(points[i]))
    lower=[]
    for i in ids:
        while len(lower)>1 and cross(points[lower[-2]],points[lower[-1]],points[i])<=0:
            lower.pop()
        lower.append(i)
    upper=[]
    for i in reversed(ids):
        while len(upper)>1 and cross(points[upper[-2]],points[upper[-1]],points[i])<=0:
            upper.pop()
        upper.append(i)
    return lower[:-1]+upper[:-1]

class Natural:
    def __init__(self,points):
        self.points=np.asarray(points)
        self.hull=hull(points)
        self.planes=[]
        for i,p in enumerate(points):
            self.planes.append([(float(2*(q[0]-p[0])),float(2*(q[1]-p[1])),float(q@q-p@p))
                                for j,q in enumerate(points) if i!=j])

    def weights(self,x):
        points=self.points
        dist2=np.sum((points-x)**2,axis=1)
        if dist2.min()<1e-26:
            return np.eye(len(points))[dist2.argmin()]
        for k,i in enumerate(self.hull):
            j=self.hull[(k+1)%len(self.hull)]
            p,q=points[i],points[j]
            edge=q-p
            cross=edge[0]*(x[1]-p[1])-edge[1]*(x[0]-p[0])
            if cross < -1e-11:
                return np.full(len(points),np.nan)
            if abs(cross)<1e-12:
                t=np.dot(x-p,edge)/np.dot(edge,edge)
                if 0<=t<=1:
                    weights=np.zeros(len(points)); weights[i]=1-t; weights[j]=t
                    return weights
        bound=1024.
        cell=[(-bound,-bound),(bound,-bound),(bound,bound),(-bound,bound)]
        for p in points:
            cell=clip(cell,2*(p[0]-x[0]),2*(p[1]-x[1]),float(p@p-x@x))
        if any(max(abs(a),abs(b))>bound*.999 for a,b in cell):
            raise ValueError('Reference clipping domain too small')
        weights=[]
        for planes in self.planes:
            stolen=cell
            for nx,ny,b in planes:
                stolen=clip(stolen,nx,ny,b)
                if not stolen: break
            weights.append(area(stolen))
        return np.array(weights)/sum(weights)

    def both_weights(self,queries):
        ordinary=np.array([self.weights(x) for x in queries])
        smooth=ordinary.copy()
        for k,x in enumerate(queries):
            lam=ordinary[k]
            r2=np.sum((self.points-x)**2,axis=1)
            if np.min(r2)<1e-26 or not np.isfinite(lam).all(): continue
            r=np.sqrt(r2)
            inverse=lam/r
            alpha=np.sum(lam*r)/inverse.sum()
            beta=np.sum(lam*r2)
            # Zero supplied gradients make xi another convex combination of colors.
            smooth[k]=(alpha*lam+beta*inverse/inverse.sum())/(alpha+beta)
        return ordinary,smooth

def shepard_weights(points,queries):
    r2=np.sum((queries[:,None,:]-points[None,:,:])**2,axis=2)
    nearest=r2.min(axis=1,keepdims=True)
    # Scaling by the minimum distance avoids large reciprocals near an anchor.
    weights=np.divide(nearest,r2,out=np.zeros_like(r2),where=r2>0)
    exact=np.where(nearest[:,0]<1e-26)[0]
    for i in exact:
        weights[i]=0; weights[i,np.argmin(r2[i])]=1
    return weights/weights.sum(axis=1,keepdims=True)

def kernel(r2):
    return .5*r2*np.log(np.maximum(r2,1e-300))

class TPS:
    def __init__(self,points,colors):
        self.points=points
        n=len(points)
        poly=np.column_stack([np.ones(n),points])
        K=kernel(np.sum((points[:,None]-points[None,:])**2,axis=2))
        self.system=np.block([[K,poly],[poly.T,np.zeros((3,3))]])
        self.coefficients=np.linalg.solve(self.system,np.vstack([colors,np.zeros((3,colors.shape[1]))]))
    def __call__(self,queries):
        k=kernel(np.sum((queries[:,None]-self.points[None,:])**2,axis=2))
        return np.column_stack([k,np.ones(len(queries)),queries])@self.coefficients

def four_triangle(points,colors,queries):
    # The probe perturbs the upper-right corner's y value through cocircularity.
    ac=points[1,1]<0
    triangles=[(0,1,2),(0,2,3)] if ac else [(0,1,3),(1,2,3)]
    out=np.full((len(queries),colors.shape[1]),np.nan)
    for ids in triangles:
        ids=list(ids)
        M=np.vstack([points[ids].T,np.ones(3)])
        weights=np.linalg.solve(M,np.column_stack([queries,np.ones(len(queries))]).T).T
        valid=(weights>=-1e-10).all(axis=1)
        out[valid]=weights[valid]@colors[ids]
    return out

def evaluate(points,colors,queries):
    nn,c1=Natural(points).both_weights(queries)
    return {'Natural neighbors':nn@colors,'Natural neighbors, flat anchors':c1@colors,
            'Shepard, inverse-square':shepard_weights(points,queries)@colors,
            'Thin-plate spline':TPS(points,colors)(queries)}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    args=parser.parse_args()
    out_dir=args.output_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    report={'scope':'CPU double-precision mathematical reference. No Pax or GPU benchmark.',
            'numpy':np.__version__,'points':POINTS.tolist(),'colors':COLORS.tolist()}
    rng=np.random.default_rng(1011)
    probes=rng.uniform(.02,.98,(128,2))
    nw,cw=Natural(POINTS).both_weights(probes)
    report['reference_checks']={
        'natural_affine_position_max_error':float(np.max(abs(nw@POINTS-probes))),
        'natural_partition_max_error':float(np.max(abs(nw.sum(axis=1)-1))),
        'c1_partition_max_error':float(np.max(abs(cw.sum(axis=1)-1))),
        'minimum_weight':float(min(nw.min(),cw.min())),
    }
    assert report['reference_checks']['natural_affine_position_max_error']<1e-7
    assert report['reference_checks']['minimum_weight']>=0
    report['anchor_max_error']={name:float(np.max(abs(values-COLORS)))
                                for name,values in evaluate(POINTS,COLORS,POINTS).items()}
    assert max(report['anchor_max_error'].values())<1e-9
    permutation=rng.permutation(len(POINTS))
    original=evaluate(POINTS,COLORS,probes[:12])
    permuted=evaluate(POINTS[permutation],COLORS[permutation],probes[:12])
    report['permutation_max_error']={k:float(np.max(abs(original[k]-permuted[k]))) for k in original}
    # Premultiplied channel bounds remain valid under positive normalized weights.
    alpha=rng.uniform(0,1,(len(POINTS),1))
    translucent=np.column_stack([rng.uniform(0,1,(len(POINTS),3))*alpha,alpha])
    report['premultiplied_violation']={}
    for name,field in evaluate(POINTS,translucent,probes).items():
        report['premultiplied_violation'][name]=float(max(0,-field.min(),(field[:,:3]-field[:,3:]).max(),field[:,3].max()-1))
    report['anchor_one_sided_x_slopes']={}
    anchor=POINTS[4]
    for step in [1e-2,1e-3,1e-4]:
        q=np.array([anchor-[step,0],anchor,anchor+[step,0]])
        values=evaluate(POINTS,COLORS,q)
        report['anchor_one_sided_x_slopes'][str(step)]={name:{
            'left_max_abs':float(np.max(abs((v[1]-v[0])/step))),
            'right_max_abs':float(np.max(abs((v[2]-v[1])/step))),
            'max_mismatch':float(np.max(abs((v[1]-v[0])/step-(v[2]-v[1])/step)))} for name,v in values.items()}
    # Small motions through a Delaunay edge flip at a square.
    points=np.array([[0.,0],[1.,0],[1.,1],[0.,1]])
    colors=np.array([[1.,0,0,1],[0,0,1,1],[1,0,0,1],[0,0,1,1]])
    query=np.array([[.5,.5],[.3,.6],[.6,.2]])
    report['edge_flip_max_channel_delta']={}
    for eps in [1e-2,1e-4,1e-6]:
        lo=points.copy();lo[1,1]=-eps
        hi=points.copy();hi[1,1]=eps
        before=evaluate(lo,colors,query);after=evaluate(hi,colors,query)
        before['Linear triangles']=four_triangle(lo,colors,query)
        after['Linear triangles']=four_triangle(hi,colors,query)
        report['edge_flip_max_channel_delta'][str(eps)]={name:float(np.max(abs(after[name]-v))) for name,v in before.items()}
    # Near collision: measure TPS amplification over a fixed square, and conditioning.
    collision_queries=rng.uniform(0,1,(400,2))
    report['tps_close_anchor_probe']=[]
    for gap in [.1,.01,.001]:
        p=np.vstack([points,[.5-gap/2,.5],[.5+gap/2,.5]])
        c=np.vstack([np.full((4,1),.5),[[0.],[1.]]])
        tps=TPS(p,c);field=tps(collision_queries)
        report['tps_close_anchor_probe'].append({'gap':gap,'matrix_condition_number':float(np.linalg.cond(tps.system)),
            'raw_min':float(field.min()),'raw_max':float(field.max())})
    # Visual comparison: same points and encoded-sRGB colors, 144 x 144 sample centers.
    size=144
    xy=(np.arange(size)+.5)/size
    xx,yy=np.meshgrid(xy,xy)
    queries=np.column_stack([xx.ravel(),yy.ravel()])
    start=time.perf_counter()
    fields=evaluate(POINTS,COLORS,queries)
    report['reference_image_seconds']=time.perf_counter()-start
    report['field_bounds']={}
    canvas=Image.new('RGB',(1000,1070),'#fafafa')
    draw=ImageDraw.Draw(canvas)
    title_font=ImageFont.load_default(size=27)
    label_font=ImageFont.load_default(size=21)
    draw.text((32,20),'Seven identical color anchors',font=title_font,fill='#222222')
    draw.text((32,57),'Independent positions · same colors · CPU reference',font=label_font,fill='#444444')
    for index,(name,field) in enumerate(fields.items()):
        report['field_bounds'][name]={'raw_min':float(field[:,:3].min()),'raw_max':float(field[:,:3].max()),
            'fraction_pixels_out_of_range':float(np.mean(((field[:,:3]<-1e-9)|(field[:,:3]>1+1e-9)).any(axis=1)))}
        rgb=np.clip(field[:,:3],0,1).reshape(size,size,3)
        left=32+(index%2)*490
        top=142+(index//2)*470
        draw.text((left,top-35),name,font=label_font,fill='#222222')
        tile=Image.fromarray(np.uint8(rgb*255)).resize((440,400),Image.Resampling.BILINEAR)
        canvas.paste(tile,(left,top))
        for p,c in zip(POINTS,COLORS):
            x=left+p[0]*439;y=top+p[1]*399
            draw.ellipse((x-5,y-5,x+5,y+5),fill=tuple(np.uint8(c[:3]*255)),outline='#333333',width=1)
    draw.text((32,1030),'Thin-plate output is clamped for display; the other three stay in range.',font=label_font,fill='#444444')
    canvas.save(out_dir/'scattered-point-comparison.png')
    # Keep generated output in the explicitly selected output directory.
    for name,field in fields.items():
        filename={'Natural neighbors':'natural','Natural neighbors, flat anchors':'natural-c1',
                  'Shepard, inverse-square':'shepard','Thin-plate spline':'tps'}[name]
        Image.fromarray(np.uint8(np.clip(field[:,:3].reshape(size,size,3),0,1)*255)).save(out_dir/f'scattered-{filename}.png')
    (out_dir/'scattered-point-results.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2),flush=True)

if __name__=='__main__':main()
