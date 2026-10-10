all: src/one.o obj/two.o
%.o: %.c
	@echo generic $< to $@ stem $*
obj/%.o: lib/%.c
	@echo obj-rule $< to $@ stem $*
