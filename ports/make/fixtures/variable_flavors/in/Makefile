a = $(b)
b = late
c := $(b)
b = later
x = one
x += two
s := one
s += $(b)
q ?= kept
q ?= not used
$(info a=[$(a)] c=[$(c)] x=[$(x)] s=[$(s)] q=[$(q)])
t = trailing   # comment
$(info t=[$(t)])
empty =
$(info empty=[$(empty)] undefined=[$(nothing_here)])
nested = b
$(info nested=[$($(nested))] braces=[${x}] one-char=[$x])
all: ; @:
