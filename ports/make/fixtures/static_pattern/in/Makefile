objs = a.o b.o
all: $(objs)
$(objs): %.o: %.c
	@echo $@ from $< stem $*
foo.x: %.o: %.c
	@echo foo.x has no prerequisites: [$^]
