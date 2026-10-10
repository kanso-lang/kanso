dirs = a b c
$(info foreach=[$(foreach d,$(dirs),$(d)/x)])
pair = $(1)-$(2)
$(info call=[$(call pair,left,right)] [$(call pair,only)])
$(info if=[$(if $(dirs),yes,no)] [$(if ,yes,no)] [$(if ,yes)])
$(info or=[$(or ,,second,third)] and=[$(and a,b,c)] [$(and a,,c)])
lazy = $(dirs)
$(info value=[$(value lazy)] origin=[$(origin lazy)] [$(origin nope)] [$(origin MAKE)])
all: ; @:
