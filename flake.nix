{
  description = "Baelyk's shell";

  inputs.crane.url = "github:ipetkov/crane";
  inputs.fenix.url = "github:nix-community/fenix";
  inputs.fenix.inputs.nixpkgs.follows = "nixpkgs";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = {
    crane,
    fenix,
    nixpkgs,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    toolchain = fenix.packages.${system}.complete.toolchain;
    craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

    # For Iced, https://github.com/iced-rs/iced/blob/master/DEPENDENCIES.md
    dlopenLibraries = with pkgs; [
      libxkbcommon
      vulkan-loader
      wayland
    ];
    rpath = nixpkgs.lib.makeLibraryPath dlopenLibraries;

    # Crane config for dependencies and workspace members
    commonArgs = {
      buildInputs = dlopenLibraries;
      strictDeps = true;
      # Only include files relevant to cargo
      src = craneLib.cleanCargoSource ./.;
    };
    # Compiled dependencies
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    # Helper function to build workspace member bins
    bin = {
      name,
      path,
      wrapInputs ? [],
    }: (commonArgs
      // {
        inherit cargoArtifacts;
        # The name of the resulting bin
        pname = name;
        # Only build the specified workspace member
        cargoExtraArgs = "-p ${name}";
        # Restrict inputs so nonrelevant modifications don't affect building,
        # e.g. so changes to `notifications` don't mean rebuilding `bar`
        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            (craneLib.fileset.commonCargoSources ./lib)
            (craneLib.fileset.commonCargoSources ./measuring-container)
            (craneLib.fileset.commonCargoSources path)
          ];
        };
        nativeBuildInputs = [
          # Needed to wrap the resulting bin
          pkgs.makeWrapper
        ];
        postFixup =
          # Patch the rpath of the resulting bin for Iced dependencies
          ''
            echo "Patching rpath..."
            rpath=$(patchelf --print-rpath $out/bin/${name})
            patchelf --set-rpath "$rpath:${rpath}" $out/bin/${name}
          ''
          # Wrap the resulting bin with runtime dependencies
          + pkgs.lib.optionalString (wrapInputs != []) ''
            echo "Wrapping bin..."
            wrapProgram $out/bin/${name} \
              --suffix PATH : ${nixpkgs.lib.makeBinPath wrapInputs}
          '';
      });
  in {
    packages.${system} = {
      bar = craneLib.buildPackage (bin {
        name = "baelyks-bar";
        path = ./bar;
        wrapInputs = [pkgs.sway pkgs.wireplumber];
      });

      launcher = craneLib.buildPackage (bin {
        name = "baelyks-launcher";
        path = ./launcher;
      });

      notifications = craneLib.buildPackage (bin {
        name = "baelyks-notification-daemon";
        path = ./notifications;
      });
    };

    devShells.${system}.default = pkgs.mkShell {
      packages = [
        toolchain
      ];

      # Patch rpath when compiling
      env.RUSTFLAGS = "-C link-arg=-Wl,-rpath,${rpath}";

      shellHook = ''
        echo $(cargo --version)
      '';
    };
  };
}
