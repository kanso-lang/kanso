import random,sys
random.seed(int(sys.argv[1]))
digs='0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ'
out=[]
for i in range(int(sys.argv[2])):
    ib=random.randint(2,36)
    lit=''.join(random.choice(digs[:min(36,ib+3)]) for _ in range(random.randint(1,8)))
    if random.random()<0.5: lit+='.'+''.join(random.choice(digs[:ib]) for _ in range(random.randint(1,5)))
    out.append('ibase=%s'%(digs[ib] if ib<36 else '10'*0+'Z'))
    out.append(lit+(' * '+lit if random.random()<0.3 else ''))
    out.append('ibase=A')
    if random.random()<0.3: out.append('obase=%d'%random.choice([2,5,7,10,16,20,36,99,1000]))
    if random.random()<0.2: out.append('scale=%d'%random.randint(0,12))
open(sys.argv[3],'w').write('\n'.join(out)+'\n')
