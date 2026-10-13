import pandas as pd, numpy as np, sys
a,b=sys.argv[1],sys.argv[2]
M={k:pd.read_csv(f'{k}/matches.csv') for k in (a,b)}
P={k:pd.read_csv(f'{k}/players.csv') for k in (a,b)}
def summ(m):
    hw=(m.hg>m.ag).mean(); d=(m.hg==m.ag).mean()
    tot=m.hg+m.ag
    r={'home goals':m.hg.mean(),'away goals':m.ag.mean(),'goals/match':tot.mean(),'sd total':tot.std(),'home win%':100*hw,'draw%':100*d,'away win%':100*(1-hw-d),
       '0-0 %':100*((m.hg==0)&(m.ag==0)).mean(),'5+ goals %':100*(tot>=5).mean()}
    for c in ['shots','sot','blocked','fouls','yel','red','corners','fk','pc','pi','tk','int']:
        r[c+'/side']=pd.concat([m['h_'+c],m['a_'+c]]).mean()
    r['pens counted/side']=pd.concat([m.h_pens,m.a_pens]).mean()
    r['home poss']=m.h_poss.mean(); r['poss sd']=m.h_poss.std(); r['subs/side']=pd.concat([m.h_subs,m.a_subs]).mean(); r['injury events']=m.injuries.mean(); r['minutes']=m.minutes.mean()
    gap=m.h_ovr-m.a_ovr; gd=m.hg-m.ag
    r['GD slope per OVR pt']=np.polyfit(gap,gd,1)[0]
    r['corr(gap,GD)']=np.corrcoef(gap,gd)[0,1]
    return r
t=pd.DataFrame({k:summ(M[k]) for k in (a,b)}).round(3); print(t.to_string())
# goal diff by gap bucket
for k in (a,b):
    m=M[k]; gap=pd.cut(m.h_ovr-m.a_ovr,[-20,-4,-2,-0.5,0.5,2,4,20])
    print(k,'home pts/match by OVR gap', m.groupby(gap,observed=True).apply(lambda x: (3*(x.hg>x.ag)+(x.hg==x.ag)).mean()).round(2).to_dict())
for k in (a,b):
    p=P[k]; p['w']=p.minutes/90; g=p.groupby('position')
    print(k,'per90'); print((g[['goals','assists','shots','sot','pa','pc','tk','int','fouls','yel']].sum().div(g.w.sum(),axis=0)).round(3).to_string())
    r=p[p.rating>0]; print(k,'ratings'); print(r.groupby('position').rating.agg(['mean','std']).round(3).to_string())
