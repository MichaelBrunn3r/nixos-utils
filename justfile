set positional-arguments
target_dir := env_var_or_default('CARGO_TARGET_DIR', './target')
is_root := `[ "$(id -u)" -eq 0 ] && echo true || echo false`

alias trim := trim-generations
trim-generations *args:
    if [ "{{ is_root }}" = "true" ]; then \
        {{ target_dir }}/debug/trim-generations "$@"; \
    else \
        cargo run -q -p trim-generations -- "$@"; \
    fi

alias nf := nanofetch
nanofetch *args:
    if [ "{{ is_root }}" = "true" ]; then \
        {{ target_dir }}/debug/nanofetch "$@"; \
    else \
        cargo run -q -p nanofetch -- "$@"; \
    fi

profile bin:
    cargo build --profile profiling -p {{ bin }}
    samply record -- {{ target_dir }}/profiling/{{ bin }}
