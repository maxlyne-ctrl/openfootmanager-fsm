import pandas as pd, glob, numpy as np
for kind in ['full','quick']:
    d=pd.concat([pd.read_csv(f) for f in glob.glob(f'season/{kind}_*.csv')],ignore_index=True)
    res=[]
    for (f,season,league),g in pd.concat([pd.read_csv(f).assign(f=f) for f in glob.glob(f'season/{kind}_*.csv')]).groupby(['f','season','league']):
        pts={}
        for hid,aid,hg,ag in zip(g.h_id,g.a_id,g.hg,g.ag):
            hp=3 if hg>ag else (1 if hg==ag else 0); ap=3 if ag>hg else (1 if hg==ag else 0)
            pts.setdefault(hid,[]).append(hp); pts.setdefault(aid,[]).append(ap)
        ppg=np.array([np.mean(v) for v in pts.values() if len(v)>=30])
        res.append((league,ppg.std(),ppg.max()*38,ppg.min()*38))
    r=pd.DataFrame(res,columns=['league','ppg_sd','champion','bottom'])
    print(f"{kind}: goals/match {(d.hg+d.ag).mean():.2f}  draws {100*(d.hg==d.ag).mean():.1f}%  home wins {100*(d.hg>d.ag).mean():.1f}%  shots {pd.concat([d.h_shots,d.a_shots]).mean():.2f}  on target {pd.concat([d.h_sot,d.a_sot]).mean():.2f}  fouls {pd.concat([d.h_fouls,d.a_fouls]).mean():.2f}  yellows {pd.concat([d.h_yel,d.a_yel]).mean():.2f}  poss sd {d.h_poss.std():.2f}")
    print(r.groupby('league')[['ppg_sd','champion','bottom']].mean().round(2).to_string())
