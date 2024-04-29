.SUFFIXES:

OBJDIR = objs

ROOT_DIR = ..

.PHONY: all
all : | $(OBJDIR)
	@echo "*** Switching to $(OBJDIR) and building ***"
	$(MAKE) -C $(OBJDIR) -f $(ROOT_DIR)/Makefile ROOT_DIR=$(ROOT_DIR) $(MAKECMDGOALS)

$(OBJDIR) ::
	@echo "*** Creating objects directory ***"
	test -d $@ || mkdir -p $@


#Do nothing rules for makefiles
Makefile : ;
%.mk : ;

%:: all
	:

.PHONY: clean
clean:
	@echo "*** Remove object directory ***"
	rm -rf $(OBJDIR)

