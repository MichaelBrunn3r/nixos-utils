target_dir := env_var_or_default('CARGO_TARGET_DIR', './target')
set positional-arguments

trim-generations *args:
    {{ target_dir }}/debug/trim-generations "$@"
