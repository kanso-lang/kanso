files := $(shell printf 'x\ny\n\n')
$(info shell=[$(files)])
$(info [$(shell echo to-stderr >&2; echo out)])
all:
	@echo $(shell echo in a recipe)
