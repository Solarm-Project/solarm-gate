#!/usr/bin/ksh

# This file and its contents are supplied under the terms of the
# Common Development and Distribution License ("CDDL"), version 1.0.
# You may only use this file in accordance with the terms of version
# 1.0 of the CDDL.
#
# A full copy of the text of the CDDL should have accompanied this
# source. A copy of the CDDL is also available via the Internet at
# http://www.illumos.org/license/CDDL.

# Copyright 2023 OpenFlowLabs GmbH.

source /lib/svc/share/smf_include.sh

if [ -z "$SMF_FMRI" ]; then
	echo "SMF framework variables are not initialised."
	exit $SMF_EXIT_ERR
fi

GARAGE=/usr/bin/garage
CONF="`svcprop -c -p config/file $SMF_FMRI`"
CONF_FILE="/etc/garage/$CONF"

[ -f "$CONF_FILE" ] || exit $SMF_EXIT_ERR_CONFIG

case "$1" in
start)
        exec ${GARAGE} \
		-c "$CONF_FILE" \
		server \
		2>&1
        ;;
*)
        echo "Unknown method."
        exit $SMF_EXIT_ERR_FATAL
        ;;
esac

exit 0

