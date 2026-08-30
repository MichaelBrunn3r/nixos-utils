{
  trim-generations, # built by the flake with a nightly (rust-overlay) toolchain
  ...
}: {
  config,
  lib,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.trim-generations;
in {
  options.services.trim-generations = {
    enable = mkEnableOption "the periodic trim-generations background service";

    package = mkOption {
      type = types.package;
      default = trim-generations;
      description = "The trim-generations package to install.";
    };

    profile = mkOption {
      type = types.path;
      default = /nix/var/nix/profiles/system;
      description = "Path to the Nix profile to trim.";
    };

    policy = {
      maxAge = mkOption {
        type = types.str;
        default = "30d";
        description = "Maximum age of generations to keep (e.g. 30d, 2w, 1M, 1y).";
      };
      keepLast = mkOption {
        type = types.ints.unsigned;
        default = 5;
        description = "Always keep the newest N old generations, even when older than maxAge.";
      };
      rules = mkOption {
        type = types.listOf types.str;
        default = ["1d*30"];
        description = "Retention rules '<duration>*<repeat>', e.g. [\"1d*7\" \"2w*10\" \"1M*12\"].";
      };
    };

    apply = mkOption {
      type = types.bool;
      default = false;
      description = "Actually trim generations. When false, run in plan mode (dry-run) only.";
    };

    schedule = mkOption {
      type = types.str;
      default = "weekly";
      description = "systemd OnCalendar schedule for the background service.";
    };

    user = mkOption {
      type = types.str;
      default = "root";
      description = "User to run the trim-generations service as.";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    systemd.services.trim-generations = {
      description = "Trim Nix profile generations";
      serviceConfig = {
        Type = "oneshot";
        User = cfg.user;
        ExecStart = lib.concatStringsSep " " (
          [
            (lib.getExe cfg.package)
            "--profile"
            (toString cfg.profile)
            "--max-age"
            cfg.policy.maxAge
            "--keep-last"
            (toString cfg.policy.keepLast)
            "--rules"
            (lib.concatStringsSep ";" cfg.policy.rules)
          ]
          ++ lib.optionals cfg.apply ["--apply"]
        );
      };
    };

    systemd.timers.trim-generations = {
      description = "Periodic trim of Nix profile generations";
      wantedBy = ["timers.target"];
      timerConfig = {
        OnCalendar = cfg.schedule;
        Persistent = true;
      };
    };
  };
}
