{
  description = "Rust";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    fenix,
    nixpkgs,
    flake-utils,
  }: let
    name = "baelyks-bar";
    displayname = "Baelyk's bar";
    version = "0.1.0";
  in
    flake-utils.lib.eachDefaultSystem (system: let
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
      packages.default =
        (pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        })
        .buildRustPackage {
          pname = name;
          version = version;

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          cargoLock.outputHashes = {
            "cryoglyph-0.1.0" = "sha256-X7S9jq8wU6g1DDNEzOtP3lKWugDnpopPDBK49iWvD4o=";
            "dpi-0.1.1" = "sha256-hlVhlQ8MmIbNFNr6BM4edKdZbe+ixnPpKm819zauFLQ=";
            "iced-0.14.0-dev" = "sha256-GqnvR6N00A8Q42R7UhNdNTrt+AQXYSPFa18ZFNUsPA0=";
            "iced_exdevtools-0.14.0-dev" = "sha256-Zw3YRoigD1CMh7a707nV/Qkj5INgwPaJpt9fQH9n95A=";
            "freedesktop-icons-0.4.0" = "sha256-bETYaRkI9TRdRVy1FmrPlbTxsygafTqZ+7e4Q6v4h8w=";
          };

          nativeBuildInputs = with pkgs; [makeWrapper];

          # For Iced, modified based on Halloy's nixpkg, then wrap for runtime deps `sway` and `wpctl`
          buildInputs = dlopenLibraries;
          postFixup = ''
            rpath=$(patchelf --print-rpath $out/bin/${name})
            patchelf --set-rpath "$rpath:${nixpkgs.lib.makeLibraryPath dlopenLibraries}" $out/bin/${name}

            wrapProgram $out/bin/${name} \
              --suffix PATH : ${nixpkgs.lib.makeBinPath [pkgs.sway pkgs.wireplumber]}
          '';
        };

      devShells.default = pkgs.mkShell {
        packages = [
          toolchain
        ];

        # For Iced, https://github.com/iced-rs/iced/blob/master/DEPENDENCIES.md
        env.RUSTFLAGS = "-C link-arg=-Wl,-rpath,${rpath}";

        shellHook = ''
          echo $(cargo --version)
        '';
      };
    })
    // flake-utils.lib.eachDefaultSystemPassThrough (system: {
      nixosModules.default = {
        config,
        lib,
        ...
      }: let
        cfg = config.services.${name};
      in {
        options = {
          services.${name} = {
            enable = lib.mkEnableOption displayname;

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${system}.default;
              defaultText = lib.literalExpression "self.pacakges.default";
              description = "Package providing {command}`${name}`.";
            };
          };
        };

        config = lib.mkIf cfg.enable {
          home.packages = [cfg.package];

          systemd.user.services.${name} = {
            Unit = {
              Description = displayname;
              PartOf = ["graphical-session.target"];
            };

            Service = {
              ExecStart = "${cfg.package}/bin/${name}";
            };

            Install = {
              WantedBy = ["graphical-session.target"];
            };
          };
        };
      };
    });
}
