one:
	@echo one
two:
	@echo two from $(firstword $(MAKEFILE_NAME) build.mk)
