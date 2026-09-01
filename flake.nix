{
  description = "NixOS utilities";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    crane,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = (nixpkgs.legacyPackages.${system}).extend rust-overlay.overlays.default;

    #region Crane setup
    craneLib = (crane.mkLib pkgs).overrideToolchain pkgs.rust-bin.nightly.latest.minimal;
    craneWorkspaceRoot = craneLib.cleanCargoSource (craneLib.path ./.);
    craneWorkspaceDeps = craneLib.buildDepsOnly {
      src = craneWorkspaceRoot;
      pname = "nixos-utils";
      version = "0.0.0";
    };
    buildWorkspaceMember = memberPath: let
      manifest = craneLib.crateNameFromCargoToml {cargoToml = memberPath + "/Cargo.toml";};
    in
      craneLib.buildPackage {
        src = craneWorkspaceRoot;
        cargoArtifacts = craneWorkspaceDeps;
        pname = manifest.pname;
        version = manifest.version;
        cargoExtraArgs = "--locked -p ${manifest.pname}";
        meta.mainProgram = manifest.pname;
      };
    #endregion Crane setup

    trim-generations = buildWorkspaceMember ./pkgs/trim-generations;
    nanofetch = buildWorkspaceMember ./pkgs/nanofetch;
  in {
    packages.${system} = {
      inherit trim-generations nanofetch; # nix build .#<package>
      default = trim-generations; # nix build .
    };
    nixosModules.trim-generations = import ./pkgs/trim-generations/mod.nix {
      inherit trim-generations pkgs;
    };
  };
}
