

OBJDIR=objs

ifndef ROOT_DIR
	include build_scripts/target.mk
else

include $(ROOT_DIR)/build_scripts/rules.mk

OBJS = main.o
SRC_DIRS = $(ROOT_DIR)


vpath %.c $(SRC_DIRS)
vpath %.h $(INCLUDE_DIRS)


.PHONY: all
all : test.hex


test.elf : $(OBJS)
	$(LD) $(LDFLAGS) $(OBJS) -o $@

-include $(OBJS:.o=.d)

endif
