$(info [$(wildcard *.c)] [$(wildcard src/*.c src/*.h)] [$(wildcard none*)] [$(wildcard a.c)])
$(info [$(wildcard ?.c)] [$(wildcard [ab].c)])
objs = $(patsubst %.c,%.o,$(wildcard *.c))
all: *.c
	@echo prereqs: $^ objs: $(objs)
