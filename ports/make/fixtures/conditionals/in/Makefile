x = 1
ifeq ($(x),1)
$(info ifeq yes)
else
$(info ifeq no)
endif
ifneq "$(x)" '2'
$(info ifneq quoted yes)
endif
ifeq ( 1,1)
$(info leading blank kept)
else ifeq (1 , 1)
$(info blanks around the comma dropped)
endif
ifdef x
$(info ifdef yes)
endif
empty =
ifndef empty
$(info empty counts as undefined)
endif
ifdef undefined_thing
$(info never)
else ifdef x
ifeq ($(x),2)
$(info never)
else
$(info nested else)
endif
endif
ifeq (a,b)
$(info $(shell echo never run >&2))
endif
all:
ifeq ($(x),1)
	@echo recipe line from a conditional
endif
	@echo after
