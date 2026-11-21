#!/bin/sh
# Starts @NAME@ from the folder this script is in, wherever the folder was extracted.
exec "$(dirname "$(readlink -f "$0")")/@NAME@" "$@"
