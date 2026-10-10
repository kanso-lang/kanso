objs := a.o b.o

all: prog

prog: $(objs)
	@echo link $@ from $^
	cat $^ > $@

%.o: %.c
	echo compile $< to $@
	cat $< > $@

.PHONY: all clean
clean:
	rm -f prog *.o
