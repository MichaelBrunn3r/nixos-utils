{
  description = "NixOS utilities";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = (nixpkgs.legacyPackages.${system}).extend rust-overlay.overlays.default;

    rustPlatform = pkgs.makeRustPlatform {
      cargo = pkgs.rust-bin.nightly.latest.minimal;
      rustc = pkgs.rust-bin.nightly.latest.minimal;
    };

    trim-generations = rustPlatform.buildRustPackage {
      pname = "trim-generations";
      version = "0.1.0";
      src = pkgs.lib.cleanSource ./.;
      cargoLock.lockFile = ./Cargo.lock;

      # Workspace may grow. Only build/install this member.
      cargoBuildFlags = ["-p" "trim-generations"];
      cargoTestFlags = ["-p" "trim-generations"];
      cargoInstallFlags = ["-p" "trim-generations"];

      meta = {
        description = "Trim a Nix profile's generations by retention policy";
        homepage = "https://github.com/MichaelBrunn3r/nixos-utils";
        mainProgram = "trim-generations";
      };
    };
  in {
    packages.${system} = {
      inherit trim-generations; # nix build .#trim-generations
      default = trim-generations; # nix build .
    };
    nixosModules.trim-generations = import ./pkgs/trim-generations/mod.nix {inherit trim-generations;};
  };
}
