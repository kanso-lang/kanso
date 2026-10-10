import random, subprocess, sys
seed=int(sys.argv[1]); n=int(sys.argv[2])
random.seed(seed)
def num():
    r=random.random()
    if r<0.3: return str(random.randint(0,99))
    if r<0.6:
        ip=str(random.randint(0,999)) if random.random()<0.7 else ''
        fp=''.join(random.choice('0123456789') for _ in range(random.randint(1,6)))
        return ip+'.'+fp
    return str(random.randint(0,10**random.randint(1,30)))
def expr(d=0):
    if d>3 or random.random()<0.3:
        x=num()
        return ('-'+x) if random.random()<0.2 else x
    op=random.choice(['+','-','*','/','%','^','sqrt','cmp','neg','len','scl'])
    if op=='^': return '(%s)^%d'%(expr(d+1),random.randint(-3,6))
    if op=='sqrt': return 'sqrt(%s)'%expr(d+1).lstrip('-')
    if op=='cmp': return '(%s %s %s)'%(expr(d+1),random.choice(['<','<=','>','>=','==','!=']),expr(d+1))
    if op=='neg': return '-(%s)'%expr(d+1)
    if op=='len': return 'length(%s)'%expr(d+1)
    if op=='scl': return 'scale(%s)'%expr(d+1)
    return '(%s %s %s)'%(expr(d+1),op,expr(d+1))
lines=[]
for i in range(n):
    if random.random()<0.15: lines.append('scale=%d'%random.randint(0,25))
    if random.random()<0.05: lines.append('obase=%d'%random.choice([2,3,8,10,10,10,16,17,25,100,1000]))
    lines.append(expr())
open(sys.argv[3],'w').write('\n'.join(lines)+'\n')
