# treadcore as a Nix flake: `nix run github:treadcore/treadcore` plays,
# `nix profile install github:treadcore/treadcore` installs, `nix
# develop` is a shell with what the build needs. nixpkgs brings its own
# Rust; nothing else is asked of the machine.
#
# The window loads Wayland, xkbcommon, the X11 libraries and the Vulkan
# loader at run time (winit and wgpu dlopen them), and NixOS has no
# global library path, so the installed `treadcore` is wrapped with
# LD_LIBRARY_PATH naming them. The Vulkan driver itself comes from the
# system (hardware.graphics on NixOS); on another distribution with Nix,
# run it through nixGL if the GPU's driver is not found.
{
  description = "treadcore: a sixteen-player overhead tank game for the drone battlefield";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      runtimeLibs = pkgs: with pkgs; [
        wayland libxkbcommon vulkan-loader
        xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr
      ];
    in {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "treadcore";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "treadcore-client" "-p" "treadcore-server" ];
          # The proofs run sixteen-bot matches for minutes: make test, not
          # every install.
          doCheck = false;
          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          buildInputs = with pkgs; [ alsa-lib udev wayland libxkbcommon vulkan-loader ];
          postInstall = ''
            install -Dm644 packaging/org.treadcore.treadcore.desktop $out/share/applications/org.treadcore.treadcore.desktop
            install -Dm644 packaging/treadcore.svg $out/share/icons/hicolor/scalable/apps/treadcore.svg
          '';
          postFixup = ''
            wrapProgram $out/bin/treadcore \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibs pkgs)}
          '';
          meta = {
            description = "A sixteen-player overhead tank game for the drone battlefield";
            homepage = "https://treadcore.org";
            license = pkgs.lib.licenses.mit;
            mainProgram = "treadcore";
            platforms = systems;
          };
        };
      });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${self.packages.${pkgs.system}.default}/bin/treadcore"; };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.system}.default ];
          packages = with pkgs; [ cargo rustc clippy rustfmt ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
        };
      });
    };
}
