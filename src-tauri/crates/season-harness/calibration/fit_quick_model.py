import pandas as pd, numpy as np, statsmodels.api as sm, statsmodels.formula.api as smf, glob, json, warnings
warnings.filterwarnings('ignore')
d=pd.concat([pd.read_csv(f) for f in glob.glob('season/full_*.csv')],ignore_index=True).dropna()
REF=dict(fin=79.0,keep=94.0,att=80.0,mid=82.0,dfn=84.0,cond=92.0)
def tac(col):
    t=d[col].str.split('|',expand=True); return t[0],t[1],t[2],t[3]
rows=[]
for s,o,home,g in [('h','a',1,'hg'),('a','h',0,'ag')]:
    p,l,te,w=tac(s+'_tac'); op,ol,ote,ow=tac(o+'_tac')
    rows.append(pd.DataFrame({'goals':d[g],'sot':d[s+'_sot'],'shots':d[s+'_shots'],'home':home,
      'fin':d[s+'_fin']-REF['fin'],'att':d[s+'_att']-REF['att'],'mid':d[s+'_mid']-REF['mid'],'cond':d[s+'_cond']-REF['cond'],
      'o_keep':d[o+'_keep']-REF['keep'],'o_def':d[o+'_def']-REF['dfn'],'o_mid':d[o+'_mid']-REF['mid'],'o_cond':d[o+'_cond']-REF['cond'],
      'style':d[s+'_style'],'o_style':d[o+'_style'],'press':p,'line':l,'tempo':te,'width':w,'o_press':op,'o_line':ol,'o_tempo':ote,'o_width':ow}))
L=pd.concat(rows,ignore_index=True)
cat=lambda v,base: f"C({v}, Treatment('{base}'))"
TERMS=' + '.join([cat('style','Balanced'),cat('o_style','Balanced'),cat('press','Medium'),cat('line','Medium'),cat('tempo','Direct'),cat('width','Normal'),
                  cat('o_press','Medium'),cat('o_line','Medium'),cat('o_tempo','Direct'),cat('o_width','Normal')])
NUM='home + fin + att + mid + cond + o_keep + o_def + o_mid + o_cond'
def clean(params):
    out={}
    for k,v in params.items():
        k2=k
        for var in ['o_style','style','o_press','press','o_line','line','o_tempo','tempo','o_width','width']:
            for base in ['Balanced','Medium','Direct','Normal']:
                k2=k2.replace(f"C({var}, Treatment('{base}'))[T.",f"{var}:")
        out[k2.rstrip(']')]=round(float(v),4)
    return out
model={'ref':REF}
for y in ['goals','sot','shots']:
    r=smf.glm(f'{y} ~ {NUM} + {TERMS}',L,family=sm.families.Poisson()).fit()
    model[y]=clean(r.params); print(y,'disp',round(r.pearson_chi2/r.df_resid,3)); print('  ',{k:v for k,v in model[y].items() if ':' not in k})
# possession, home perspective
H=pd.DataFrame({'poss':d.h_poss,'mid_diff':d.h_mid-d.a_mid,'att_diff':d.h_att-d.a_att,'style':d.h_style,'o_style':d.a_style})
for c,(p,l,te,w) in {'':tac('h_tac'),'o_':tac('a_tac')}.items():
    H[c+'press']=p;H[c+'line']=l;H[c+'tempo']=te;H[c+'width']=w
r=smf.ols(f'poss ~ mid_diff + att_diff + {TERMS}',H).fit(); model['poss']=clean(r.params); model['poss']['noise']=round(float(np.sqrt(r.mse_resid)),3)
print('poss',{k:v for k,v in model['poss'].items() if ':' not in k})
json.dump(model,open('model_v2.json','w'),indent=1)
print(json.dumps({k:{kk:vv for kk,vv in v.items() if ':' in kk} for k,v in model.items() if k!='ref'})[:1500])
