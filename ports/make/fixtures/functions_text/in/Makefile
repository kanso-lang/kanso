f = src/a.c src/b.h c.c d
$(info subst=[$(subst .c,.o,$(f))])
$(info patsubst=[$(patsubst %.c,%.o,$(f))])
$(info strip=[$(strip   a   b  )])
$(info findstring=[$(findstring b.h,$(f))] [$(findstring z,$(f))])
$(info filter=[$(filter %.c %.h,$(f))] filter-out=[$(filter-out %.c,$(f))])
$(info sort=[$(sort b a c b a)])
$(info word=[$(word 2,$(f))] words=[$(words $(f))] wordlist=[$(wordlist 2,3,$(f))])
$(info firstword=[$(firstword $(f))] lastword=[$(lastword $(f))])
$(info dir=[$(dir $(f))] notdir=[$(notdir $(f))])
$(info suffix=[$(suffix $(f))] basename=[$(basename $(f))])
$(info addprefix=[$(addprefix lib/,a b)] addsuffix=[$(addsuffix .x,a b)])
$(info join=[$(join a b c,1 2)])
all: ; @:
