import sys,subprocess,shutil,os
src='lx/lx/lx.kso'
s=open(src).read()
blocks=s.split('\n\n')
def heads():
    out=[]
    for i,b in enumerate(blocks):
        first=[l for l in b.split('\n') if not l.startswith('#')]
        if first and first[0].startswith('fn scan_char'): out.append(i)
    return out
sc=heads()
keepset=set(int(x) for x in sys.argv[1].split(',')) if len(sys.argv)>1 and sys.argv[1] else set()
outb=[b for i,b in enumerate(blocks) if i not in sc or sc.index(i) in keepset]
open('s/lx/lx.kso','w').write('\n\n'.join(outb))
r=subprocess.run([os.environ['K'],'run','.','--interp'],cwd='s',capture_output=True,text=True)
print(len(sc), (r.stdout+r.stderr).splitlines()[0][:90])
