#!/usr/bin/env bash
# Quote first for Exec's argument parser, then for desktop-entry string storage.
desktop_exec() {
	local value=$1
	# shortcut: GIO cannot resolve field codes in executable paths; reject % until a trampoline is justified.
	if [[ $value =~ [[:cntrl:]] || $value == *%* || $value == *=* ]]; then
		printf 'Executable paths must not contain control characters, percent, or equals signs.\n' >&2
		return 2
	fi
	value=${value//\\/\\\\}
	value=${value//\"/\\\"}
	value=${value//\$/\\\$}
	value=${value//\`/\\\`}
	value="\"$value\""
	value=${value//\\/\\\\}
	printf '%s' "$value"
}
