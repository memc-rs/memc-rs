# Minimal Makefile for building testapp

CC = gcc
CFLAGS = -DHAVE_CONFIG_H -m64 -g -O0 -pthread -Wall -pedantic -Wmissing-prototypes -Wmissing-declarations -Wredundant-decls -I.
LIBS = -levent

# Sources for timedrun
timedrun_SOURCES = timedrun.c

# Objects derived from sources (excluding headers)
timedrun_OBJECTS = timedrun.o

# Sources for testapp
testapp_SOURCES = cache.c crc32c.c jenkins_hash.c\
									testapp.c \
									murmur3_hash.c slabs.c stats_prefix.c util.c

# Objects derived from sources (excluding headers)
testapp_OBJECTS = testapp.o util.o stats_prefix.o jenkins_hash.o murmur3_hash.o cache.o crc32c.o

# Build graph:
# testapp depends on testapp_OBJECTS
# Each .o depends on its .c file and included headers
# testapp.o: testapp.c (no headers listed)
# util.o: util.c util.h
# stats_prefix.o: stats_prefix.c stats_prefix.h
# jenkins_hash.o: jenkins_hash.c hash.h
# murmur3_hash.o: murmur3_hash.c hash.h
# cache.o: cache.c cache.h (assuming cache.h exists)
# crc32c.o: crc32c.c crc32c.h (assuming crc32c.h exists)

# Compilation rule
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Default target
all: testapp timedrun

# Linking rule for testapp
testapp: $(testapp_OBJECTS)
	$(CC) $(CFLAGS) -o $@ $^ $(LIBS)

timedrun: $(timedrun_OBJECTS)
	$(CC) $(CFLAGS) -o $@ $^ $(LIBS)

# Clean target
clean:
	rm -f $(testapp_OBJECTS) $(timedrun_OBJECTS) testapp timedrun

.PHONY: clean all
