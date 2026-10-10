import random, sys
random.seed(int(sys.argv[1])); n=int(sys.argv[2])
def arg(lo,hi,dec=4):
    v=random.uniform(lo,hi)
    return ('%.'+str(random.randint(0,dec))+'f')%v
out=[]
for i in range(n):
    if random.random()<0.2: out.append('scale=%d'%random.randint(0,40))
    f=random.choice('scale'+'j')
    if f=='s' or f=='c': out.append('%s(%s)'%(f,arg(-50,50)))
    elif f=='a': out.append('a(%s)'%arg(-1000,1000))
    elif f=='l': out.append('l(%s)'%arg(0.001,100000))
    elif f=='e': out.append('e(%s)'%arg(-30,60))
    else: out.append('j(%d,%s)'%(random.randint(-5,5),arg(-15,15)))
open(sys.argv[3],'w').write('\n'.join(out)+'\n')
