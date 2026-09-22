import re
TOK=re.compile(r'\(|\)|"(?:\\.|[^"\\])*"|[^\s()]+')
def parse(t):
    st=[[]]
    for m in TOK.finditer(t):
        s=m.group(0)
        if s=='(': st.append([])
        elif s==')': x=st.pop(); st[-1].append(x)
        else: st[-1].append(s)
    return st[0]
def dump(x,ind=0):
    if not isinstance(x,list): return x
    if all(not isinstance(e,list) for e in x): return "("+" ".join(x)+")"
    head=[e for e in x if not isinstance(e,list)]
    out="("+" ".join(head)
    for e in x:
        if isinstance(e,list): out+="\n"+"\t"*(ind+1)+dump(e,ind+1)
    return out+")"
def find_sym(lib,name):
    tree=parse(open(f"/usr/share/kicad/symbols/{lib}.kicad_sym").read())[0]
    for e in tree:
        if isinstance(e,list) and e[0]=='symbol' and e[1]==f'"{name}"': return e
def pins(sym):
    out={}
    def walk(n):
        if isinstance(n,list):
            if n and n[0]=='pin':
                at=[e for e in n if isinstance(e,list) and e[0]=='at'][0]
                num=[e for e in n if isinstance(e,list) and e[0]=='number'][0][1].strip('"')
                nm=[e for e in n if isinstance(e,list) and e[0]=='name'][0][1].strip('"')
                out[num]=(float(at[1]),float(at[2]),int(float(at[3])),nm)
            for c in n: walk(c)
    walk(sym); return out
def extends(sym):
    for e in sym:
        if isinstance(e,list) and e[0]=='extends': return e[1].strip('"')
