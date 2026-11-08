typeset -g _tipun_command=
typeset -g _tipun_expanded=

_tipun_preexec() {
    _tipun_command=$1
    _tipun_expanded=$3
}

_tipun_precmd() {
    local exit_code=$?
    emulate -L zsh
    [[ -n $_tipun_command ]] || return 0
    local typed=$_tipun_command expanded=$_tipun_expanded
    _tipun_command=
    _tipun_expanded=
    command tipun analyze --command="$typed" --expanded="$expanded" --exit-code="$exit_code" \
        --columns="$COLUMNS"
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec _tipun_preexec
add-zsh-hook precmd _tipun_precmd
