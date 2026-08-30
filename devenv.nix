{pkgs, ...}: {
  languages.rust = {
    enable = true;
    channel = "nightly";
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };

  packages = with pkgs; [
    git
  ];

  git-hooks.hooks = {
    clippy.enable = true;
    rustfmt.enable = true;
  };
}
