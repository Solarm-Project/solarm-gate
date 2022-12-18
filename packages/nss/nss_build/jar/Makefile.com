#
# CDDL HEADER START
#
# The contents of this file are subject to the terms of the
# Common Development and Distribution License, Version 1.0 only
# (the "License").  You may not use this file except in compliance
# with the License.
#
# You can obtain a copy of the license at usr/src/OPENSOLARIS.LICENSE
# or http://www.opensolaris.org/os/licensing.
# See the License for the specific language governing permissions
# and limitations under the License.
#
# When distributing Covered Code, include this CDDL HEADER in each
# file and include the License file at usr/src/OPENSOLARIS.LICENSE.
# If applicable, add the following below this CDDL HEADER, with the
# fields enclosed by brackets "[]" replaced with your own identifying
# information: Portions Copyright [yyyy] [name of copyright owner]
#
# CDDL HEADER END
#

#
# Copyright 2017 Hayashi Naoyuki
#

TOP= ../../..

LIBRARY = libjar.a
VERS = .1
OBJECTS = \
	jarver.o \
	jarsign.o \
	jar.o \
	jar-ds.o \
	jarfile.o \
	jarint.o
include ../../Makefile.nss

HDRDIR=		$(NSS_BASE)/lib/jar
SRCDIR=		$(NSS_BASE)/lib/jar

LIBS =		$(LIBRARY)

MAPFILE=
CPPFLAGS +=
CPPFLAGS += -DMOZILLA_CLIENT=1
CFLAGS +=
LDLIBS +=
ROOTLIBS=
ROOTLINKS=

all: $(LIBS)
install: all $(ROOTLIBS) $(ROOTLINKS)

include $(SRC)/lib/Makefile.targ
