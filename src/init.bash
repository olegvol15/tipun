_tipun_history=-

_tipun_prompt() {
    local exit_code=$? entry number
    entry=$(HISTTIMEFORMAT= builtin history 1)
    entry=${entry#"${entry%%[! ]*}"}
    number=${entry%%[!0-9]*}
    # A new history number means a command ran; Enter on an empty line and lines left out of
    # history keep the old one. The first prompt only records it.
    if [[ -n $number && $number != "$_tipun_history" && $_tipun_history != - ]]; then
        command tipun analyze --command="${entry#"$number"??}" --aliases="$(builtin alias)" \
            --exit-code="$exit_code" --columns="${COLUMNS-}"
    fi
    _tipun_history=$number
    return "$exit_code"
}

case ${PROMPT_COMMAND-} in
    *_tipun_prompt*) ;;
    *) PROMPT_COMMAND="_tipun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
esac
