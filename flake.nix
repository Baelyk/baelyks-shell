{
  description = "Baelyk's shell";

  inputs.fenix.url = "github:nix-community/fenix";
  inputs.fenix.inputs.nixpkgs.follows = "nixpkgs";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = {
    self,
    fenix,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    toolchain = fenix.packages.${system}.stable.toolchain;
    # For Iced, https://github.com/iced-rs/iced/blob/master/DEPENDENCIES.md
    dlopenLibraries = with pkgs; [
      libxkbcommon
      vulkan-loader
      wayland
    ];
    rpath = nixpkgs.lib.makeLibraryPath dlopenLibraries;
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = [
        toolchain
        pkgs.llvmPackages.bintools
      ];

      # For Iced, https://github.com/iced-rs/iced/blob/master/DEPENDENCIES.md
      env.RUSTFLAGS = "-C link-self-contained=-linker -C link-arg=-Wl,-rpath,${rpath}";

      shellHook = ''
        echo $(cargo --version)
      '';
    };
  };
}
