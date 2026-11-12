_tipun_history=-
_tipun_aliases=
_tipun_trusted=

_tipun_prompt() {
    local exit_code=$? entry number
    entry=$(HISTTIMEFORMAT= builtin history 1)
    entry=${entry#"${entry%%[! ]*}"}
    number=${entry%%[!0-9]*}
    # A new history number means a command ran; Enter on an empty line and lines left out of
    # history keep the old one. The first prompt only records it.
    if [[ -n $number && $number != "$_tipun_history" && $_tipun_history != - && -n $_tipun_trusted ]]
    then
        command tipun analyze --command="${entry#"$number"??}" --aliases="$_tipun_aliases" \
            --exit-code="$exit_code" --columns="${COLUMNS-}"
    fi
    _tipun_history=$number
    _tipun_trusted=
    return "$exit_code"
}

# bash expands aliases when it reads a line, so the next command uses the aliases left by the last
# prompt hook. A snapshot taken while another hook still follows is not trusted.
_tipun_snapshot() {
    local exit_code=$? last=${PROMPT_COMMAND-}
    if ((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 501)); then
        last=${PROMPT_COMMAND[${#PROMPT_COMMAND[@]} - 1]-}
    fi
    if [[ $last == *_tipun_snapshot ]]; then
        _tipun_aliases=$(builtin alias)
        _tipun_trusted=1
    fi
    return "$exit_code"
}

case " ${PROMPT_COMMAND[*]-} " in
    *_tipun_prompt*) ;;
    *)
        if ((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 501)); then
            PROMPT_COMMAND=(_tipun_prompt ${PROMPT_COMMAND[@]+"${PROMPT_COMMAND[@]}"} _tipun_snapshot)
        else
            PROMPT_COMMAND=_tipun_prompt$'\n'${PROMPT_COMMAND-}$'\n'_tipun_snapshot
        fi
        ;;
esac
