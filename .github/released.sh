#!/bin/sh
# Prints "true" if the tag $1 exists on origin, "false" if it doesn't, and
# fails if origin can't be asked: a failed check must not look unreleased.
git ls-remote --exit-code --tags origin "refs/tags/$1" >/dev/null
case $? in
    0) echo true ;;
    2) echo false ;;
    *) echo "::error::Can't check origin for tag $1" >&2; exit 1 ;;
esac
